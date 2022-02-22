use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use grep_matcher::{Match, Matcher};
use grep_searcher::{
    LineIter, LineStep, Searcher, Sink, SinkFinish, SinkMatch,
};

use crate::counter::CounterWriter;
use crate::stats::Stats;
use crate::util::{find_iter_at_in_context, Replacer};
use crate::PrinterPath;

/// The configuration for the Diff printer.
///
/// This is manipulated by the DiffBuilder and then referenced by the actual
/// implementation. Once a printer is built, the configuration is frozen and
/// cannot changed.
#[derive(Debug, Clone)]
struct Config {
    replacement: Arc<Vec<u8>>,
}

impl Default for Config {
    fn default() -> Config {
        Config { replacement: Arc::new(vec![]) }
    }
}

/// A builder for a Diff lines printer.
///
/// The builder permits configuring how the printer behaves. The Diff printer
/// requires a replacement to be meaningful, and the output is pretty much
/// non-configurable.
///
/// Line numbers need to be present, but context lines are not dealt with at
/// the moment, as they require some kind of logic to buffer the output until
/// the header is known (since the amount of context lines affect its contents
/// and needs to be printed before the context lines).
///
/// Once a `Diff` printer is built, its configuration cannot be changed.
#[derive(Clone, Debug)]
pub struct DiffBuilder {
    config: Config,
}

impl DiffBuilder {
    /// Return a new builder for configuring the Diff printer.
    pub fn new() -> DiffBuilder {
        DiffBuilder { config: Config::default() }
    }

    /// Create a Diff printer that writes results to the given writer.
    pub fn build<W: io::Write>(&self, wtr: W) -> Diff<W> {
        Diff {
            config: self.config.clone(),
            wtr: CounterWriter::new(wtr),
            matches: vec![],
        }
    }

    /// Set the bytes that will be used to replace each occurrence of a match
    /// found.
    ///
    /// The replacement bytes given may include references to capturing groups,
    /// which may either be in index form (e.g., `$2`) or can reference named
    /// capturing groups if present in the original pattern (e.g., `$foo`).
    ///
    /// For documentation on the full format, please see the `Capture` trait's
    /// `interpolate` method in the
    /// [grep-printer](https://docs.rs/grep-printer) crate.
    pub fn replacement(&mut self, replacement: Vec<u8>) -> &mut DiffBuilder {
        self.config.replacement = Arc::new(replacement);
        self
    }
}

/// The Diff printer, which emits search & replace info in unified diff format.
#[derive(Debug)]
pub struct Diff<W> {
    config: Config,
    wtr: CounterWriter<W>,
    matches: Vec<Match>,
}

impl<W: io::Write> Diff<W> {
    /// Return a Diff lines printer with a default configuration that writes
    /// matches to the given writer.
    pub fn new(wtr: W) -> Diff<W> {
        DiffBuilder::new().build(wtr)
    }

    /// Return an implementation of `Sink` associated with a file path.
    ///
    /// When the printer is associated with a path, then it may, depending on
    /// its configuration, print the path along with the matches found.
    pub fn sink_with_path<'p, 's, M, P>(
        &'s mut self,
        matcher: M,
        path: &'p P,
    ) -> DiffSink<'p, 's, M, W>
    where
        M: Matcher,
        P: ?Sized + AsRef<Path>,
    {
        DiffSink {
            matcher,
            replacer: Replacer::new(),
            diff: self,
            path: path.as_ref(),
            start_time: Instant::now(),
            match_count: 0,
            b_line_offset: 0,
            after_context_remaining: 0,
            binary_byte_offset: None,
            begin_printed: false,
            stats: Stats::new(),
        }
    }

    /// Write the given line in the diff output as a removed line.
    /// The line needs to include the (original) line terminator.
    fn write_unidiff_removed(&mut self, line: &[u8]) -> io::Result<()> {
        self.wtr.write(&[b'-'])?;
        self.wtr.write(line)?;
        Ok(())
    }

    /// Write the given line in the diff output as an added line.
    /// The line needs to include the (original) terminator.
    fn write_unidiff_added(&mut self, line: &[u8]) -> io::Result<()> {
        self.wtr.write(&[b'+'])?;
        self.wtr.write(line)?;
        Ok(())
    }

    /// Write an empty line that separates the diff entries.
    fn write_unidiff_hunk_header(
        &mut self,
        a_ln: u64,
        a_count: u64,
        b_ln: u64,
        b_count: u64,
    ) -> io::Result<()> {
        self.wtr.write(
            format!("@@ -{},{} +{},{} @@\n", a_ln, a_count, b_ln, b_count)
                .as_bytes(),
        )?;
        Ok(())
    }

    /// Write an empty line that separates the diff entries:
    ///   ripgrep
    ///   --- path/to/a
    ///   +++ path/to/b
    fn write_unidiff_header(&mut self, path: &[u8]) -> io::Result<()> {
        self.wtr.write(b"ripgrep\n")?;
        self.wtr.write(b"--- ")?;
        self.wtr.write(path)?;
        self.wtr.write(&[b'\n'])?;
        self.wtr.write(b"+++ ")?;
        self.wtr.write(path)?;
        self.wtr.write(&[b'\n'])?;
        Ok(())
    }
}

impl<W> Diff<W> {
    /// Returns true if and only if this printer has written at least one byte
    /// to the underlying writer during any of the previous searches.
    pub fn has_written(&self) -> bool {
        self.wtr.total_count() > 0
    }

    /// Return a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.wtr.get_mut()
    }

    /// Consume this printer and return back ownership of the underlying
    /// writer.
    pub fn into_inner(self) -> W {
        self.wtr.into_inner()
    }
}

/// An implementation of `Sink` associated with a matcher and an optional file
/// path for the Diff printer.
///
/// This type is generic over a few type parameters:
///
/// * `'p` refers to the lifetime of the file path, if one is provided. When
///   no file path is given, then this is `'static`.
/// * `'s` refers to the lifetime of the
///   [`Diff`](struct.Diff.html)
///   printer that this type borrows.
/// * `M` refers to the type of matcher used by
///   `grep_searcher::Searcher` that is reporting results to this sink.
/// * `W` refers to the underlying writer that this printer is writing its
///   output to.
#[derive(Debug)]
pub struct DiffSink<'p, 's, M: Matcher, W> {
    matcher: M,
    replacer: Replacer<M>,
    diff: &'s mut Diff<W>,
    path: &'p Path,
    start_time: Instant,
    match_count: u64,
    b_line_offset: i64,
    after_context_remaining: u64,
    binary_byte_offset: Option<u64>,
    begin_printed: bool,
    stats: Stats,
}

impl<'p, 's, M: Matcher, W: io::Write> DiffSink<'p, 's, M, W> {
    /// Returns true if and only if this printer received a match in the
    /// previous search.
    ///
    /// This is unaffected by the result of searches before the previous
    /// search.
    pub fn has_match(&self) -> bool {
        self.match_count > 0
    }

    /// Return the total number of matches reported to this sink.
    ///
    /// This corresponds to the number of times `Sink::matched` is called.
    pub fn match_count(&self) -> u64 {
        self.match_count
    }

    /// If binary data was found in the previous search, this returns the
    /// offset at which the binary data was first detected.
    ///
    /// The offset returned is an absolute offset relative to the entire
    /// set of bytes searched.
    ///
    /// This is unaffected by the result of searches before the previous
    /// search. e.g., If the search prior to the previous search found binary
    /// data but the previous search found no binary data, then this will
    /// return `None`.
    pub fn binary_byte_offset(&self) -> Option<u64> {
        self.binary_byte_offset
    }

    /// Return a reference to the stats produced by the printer for all
    /// searches executed on this sink.
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Execute the matcher over the given bytes and record the match
    /// locations if the current configuration demands match granularity.
    fn record_matches(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.diff.matches.clear();
        // If printing requires knowing the location of each individual match,
        // then compute and stored those right now for use later. While this
        // adds an extra copy for storing the matches, we do amortize the
        // allocation for it and this greatly simplifies the printing logic to
        // the extent that it's easy to ensure that we never do more than
        // one search to find the matches.
        let matches = &mut self.diff.matches;
        find_iter_at_in_context(
            searcher,
            &self.matcher,
            bytes,
            range.clone(),
            |m| {
                let (s, e) = (m.start() - range.start, m.end() - range.start);
                matches.push(Match::new(s, e));
                true
            },
        )?;
        // Don't report empty matches appearing at the end of the bytes.
        if !matches.is_empty()
            && matches.last().unwrap().is_empty()
            && matches.last().unwrap().start() >= bytes.len()
        {
            matches.pop().unwrap();
        }
        Ok(())
    }

    /// If the configuration specifies a replacement, then this executes the
    /// replacement, lazily allocating memory if necessary.
    ///
    /// To access the result of a replacement, use `replacer.replacement()`.
    fn replace(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.replacer.clear();
        let replacement = (*self.diff.config.replacement).as_ref();
        self.replacer.replace_all(
            searcher,
            &self.matcher,
            bytes,
            range,
            replacement,
        )?;
        Ok(())
    }

    /// Write the header information which contains the path of the
    /// source and destination file of the diff.
    fn write_header(&mut self) -> io::Result<()> {
        if self.begin_printed {
            return Ok(());
        }
        let ppath = PrinterPath::with_separator(self.path, None);
        self.diff.write_unidiff_header(&ppath.as_bytes())?;
        self.begin_printed = true;
        Ok(())
    }
}

impl<'p, 's, M: Matcher, W: io::Write> Sink for DiffSink<'p, 's, M, W> {
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        self.write_header()?;

        self.match_count += 1;
        // When we've exceeded our match count, then the remaining context
        // lines should not be reset, but instead, decremented. This avoids a
        // bug where we display more matches than a configured limit. The main
        // idea here is that 'matched' might be called again while printing
        // an after-context line. In that case, we should treat this as a
        // contextual line rather than a matching line for the purposes of
        // termination.
        self.after_context_remaining = searcher.after_context() as u64;

        self.record_matches(
            searcher,
            mat.buffer(),
            mat.bytes_range_in_buffer(),
        )?;
        self.replace(searcher, mat.buffer(), mat.bytes_range_in_buffer())?;
        self.stats.add_matches(self.diff.matches.len() as u64);
        self.stats.add_matched_lines(mat.lines().count() as u64);

        // Entire search (a) and replacement (b) contents.
        let a_bytes = mat.bytes();
        let (b_bytes, _) = self.replacer.replacement().unwrap();

        // To get the correct number of lines removed added without any
        // assumptions about single or multi line search/replace, just
        // loop over lines here and count them.
        let a_line_number = mat.line_number().unwrap();
        let b_line_number =
            (self.b_line_offset + (a_line_number as i64)) as u64;
        let line_term = searcher.line_terminator().as_byte();
        let mut a_stepper = LineStep::new(line_term, 0, a_bytes.len());
        let mut b_stepper = LineStep::new(line_term, 0, b_bytes.len());
        let mut a_count: u64 = 0;
        let mut b_count: u64 = 0;
        while let Some(_) = a_stepper.next(a_bytes) {
            a_count += 1;
        }
        while let Some(_) = b_stepper.next(b_bytes) {
            b_count += 1;
        }

        // When a replacement has different line count, the offset for later
        // replacements is affected as the destination line count is relative
        // to the already inserted new lines.
        self.b_line_offset += (b_count as i64) - (a_count as i64);

        // header of a replacement contains the line number offset in
        // the source (a) and destination (b) files, as well as the
        // number of lines removed (a_count) / added (b_count).
        self.diff.write_unidiff_hunk_header(
            a_line_number,
            a_count,
            b_line_number,
            b_count,
        )?;

        // When printing the actual lines, a -/+ sign is prefixed for
        // each line, so we need to output our match/replace chunks line
        // by line and insert the proper prefix.
        let a_lines = LineIter::new(line_term, a_bytes);
        for line in a_lines {
            self.diff.write_unidiff_removed(line)?;
        }
        let b_lines = LineIter::new(line_term, b_bytes);
        for line in b_lines {
            self.diff.write_unidiff_added(line)?;
        }

        Ok(true)
    }

    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, io::Error> {
        self.diff.wtr.reset_count();
        self.start_time = Instant::now();
        self.match_count = 0;
        self.b_line_offset = 0;
        self.after_context_remaining = 0;
        self.binary_byte_offset = None;
        Ok(true)
    }

    fn finish(
        &mut self,
        _searcher: &Searcher,
        finish: &SinkFinish,
    ) -> Result<(), io::Error> {
        if !self.begin_printed {
            return Ok(());
        }

        self.binary_byte_offset = finish.binary_byte_offset();
        self.stats.add_elapsed(self.start_time.elapsed());
        self.stats.add_searches(1);
        if self.match_count > 0 {
            self.stats.add_searches_with_match(1);
        }
        self.stats.add_bytes_searched(finish.byte_count());
        self.stats.add_bytes_printed(self.diff.wtr.count());

        Ok(())
    }
}
