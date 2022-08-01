use std::io;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;
use std::time::Instant;

use grep_matcher::{Matcher, Match};
use grep_searcher::{Searcher, Sink, SinkContextKind, SinkMatch, SinkContext, SinkFinish};

use crate::counter::CounterWriter;
use crate::patcht::{PatchHunk, PatchStyle};
use crate::util::{find_iter_at_in_context, Replacer};

const ORIG_PREFIX: &[u8] = b"--- ";
const MOD_PREFIX: &[u8] = b"+++ ";

#[derive(Debug, Clone)]
struct Config {
    // Patch printing can only be used with a replacement.
    replacement: Vec<u8>,
    style: PatchStyle,
}

impl Default for Config {
    fn default() -> Config {
        Config { style: PatchStyle::Unified, replacement: Vec::default(), }
    }
}

/// Configuration for the patch-output printer
#[derive(Clone, Debug, Default)]
pub struct PatchBuilder {
    config: Config,
}

impl PatchBuilder {
    /// Return a new builder for configuring the patch printer.
    pub fn new() -> PatchBuilder {
        PatchBuilder::default()
    }

    /// Create a Patch printer that writes results to the given writer.
    pub fn build<W: io::Write>(&self, wtr: W) -> Patch<W> {
        Patch {
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
    pub fn replacement(
        &mut self,
        replacement: Vec<u8>,
    ) -> &mut PatchBuilder {
        self.config.replacement = replacement;
        self
    }
}

/// A printer for generating patch output, usable with the POSIX `patch`
/// utility.
#[derive(Debug)]
pub struct Patch<W> {
    config: Config,
    wtr: CounterWriter<W>,
    matches: Vec<Match>,
}

impl<W> Patch<W> {
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
/// path for the patch printer.
///
/// A `Sink` can be created via the
/// [`Standard::sink`](struct.Standard.html#method.sink)
/// or
/// [`Standard::sink_with_path`](struct.Standard.html#method.sink_with_path)
/// methods, depending on whether you want to include a file path in the
/// printer's output.
#[derive(Debug)]
pub struct PatchSink<'p, 's, M: Matcher, W> {
    matcher: M,
    patch: &'s mut Patch<W>,
    replacer: Replacer<M>,
    current_hunk: Option<PatchHunk>,
    path: &'p Path,
    start_time: Instant,
    match_count: u64,
    after_context_remaining: u64,
    binary_byte_offset: Option<u64>,
    begin_printed: bool,
}

impl<'p, 's, M: Matcher, W: io::Write> PatchSink<'p, 's, M, W> {
    /// Returns true if and only if this printer received a match in the
    /// previous search.
    ///
    /// This is unaffected by the result of searches before the previous
    /// search on this sink.
    pub fn has_match(&self) -> bool {
        return self.match_count > 0;
    }

    /// Execute the matcher over the given bytes and record the match
    /// locations if the current configuration demands match granularity.
    fn record_matches(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.patch.matches.clear();

        // Implementation taken from `Standard.record_matches`; see comment
        // there about allocation
        let matches = &mut self.patch.matches;
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
            && matches.last().unwrap().start() >= range.end
        {
            matches.pop().unwrap();
        }
        Ok(())
    }

    /// The Patch printer performs replacement unconditionally.
    fn replace(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.replacer.clear();
        self.replacer.replace_all(
            searcher,
            &self.matcher,
            bytes,
            range,
            &self.patch.config.replacement,
        )
    }

    /// Write the patch header, which includes the name and timestamp of the
    /// current file
    fn write_patch_header(&mut self) -> io::Result<()> {
        if self.begin_printed {
            return Ok(());
        }
        write_header(&mut self.patch.wtr, self.path)?;
        self.begin_printed = true;
        Ok(())
    }
}

fn write_header<W: io::Write>(wtr: &mut W, path: &Path) -> io::Result<()> {
    let path_bytes = path.as_os_str().as_bytes();
    wtr.write(ORIG_PREFIX)?;
    wtr.write(path_bytes)?;
    // The GNU and POSIX documentation both state that diffs include file
    // timestamps, but git doesn't include one with either `diff` or
    // `format-patch`, and indeed GNU `patch` doesn't seem to need timestamps.
    // (Haven't checked BSD but I'd be surprised if it's different in this
    // regard.)

    // XXX should the line-endings for patch files match the native line-endings?
    // Will this be done automatically by the `BufferWriter`?
    wtr.write(&[b'\n'])?;
    wtr.write(MOD_PREFIX)?;
    wtr.write(path_bytes)?;
    wtr.write(&[b'\n'])?;
    Ok(())
}

impl<'p, 's, M: Matcher, W: io::Write> Sink for PatchSink<'p, 's, M, W> {
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        self.write_patch_header()?;

        self.match_count += 1;

        self.record_matches(
            searcher,
            mat.buffer(),
            mat.bytes_range_in_buffer(),
        )?;
        self.replace(searcher, mat.buffer(), mat.bytes_range_in_buffer())?;

        if searcher.binary_detection().convert_byte().is_some() {
            if self.binary_byte_offset.is_some() {
                return Ok(false);
            }
        }

        let hunk = self.current_hunk.get_or_insert(PatchHunk::default());
        hunk.add_match(
            mat, self.replacer.replacement().expect("no replacement occurred").0);

        Ok(true)
    }

    fn context(
        &mut self,
        searcher: &Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, io::Error> {
        self.patch.matches.clear();
        self.replacer.clear();

        if ctx.kind() == &SinkContextKind::After {
            self.after_context_remaining =
                self.after_context_remaining.saturating_sub(1);
        }
        if searcher.binary_detection().convert_byte().is_some() {
            if self.binary_byte_offset.is_some() {
                return Ok(false);
            }
        }

        let hunk = self.current_hunk.get_or_insert(PatchHunk::default());
        hunk.add_context(ctx);

        return Ok(true)
    }

    fn context_break(
        &mut self,
        _: &Searcher,
    ) -> Result<bool, io::Error> {
        if let Some(previous) = &mut self.current_hunk {
            previous.write(&mut self.patch.wtr, self.patch.config.style)?;
        }
        self.current_hunk = Some(PatchHunk::default());
        Ok(true)
    }

    fn binary_data(
        &mut self,
        _searcher: &Searcher,
        binary_byte_offset: u64,
    ) -> Result<bool, io::Error> {
        self.binary_byte_offset = Some(binary_byte_offset);
        Ok(true)
    }


    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, io::Error> {
        self.patch.wtr.reset_count();
        self.start_time = Instant::now();
        self.match_count = 0;
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

        if let Some(previous) = &mut self.current_hunk {
            previous.write(&mut self.patch.wtr, self.patch.config.style)?;
        }

        self.binary_byte_offset = finish.binary_byte_offset();

        Ok(())
    }
}

impl<W: io::Write> Patch<W> {
    /// Return an implementation of `Sink` associated with a file path.
    ///
    /// When the printer is associated with a path, then it may, depending on
    /// its configuration, print the path along with the matches found.
    pub fn sink_with_path<'p, 's, M, P>(
        &'s mut self,
        matcher: M,
        path: &'p P,
    ) -> PatchSink<'p, 's, M, W>
    where
        M: Matcher,
        P: ?Sized + AsRef<Path>,
    {
        PatchSink {
            matcher: matcher,
            patch: self,
            replacer: Replacer::new(),
            current_hunk: None,
            path: path.as_ref(),
            start_time: Instant::now(),
            match_count: 0,
            after_context_remaining: 0,
            binary_byte_offset: None,
            begin_printed: false,
        }
    }


}