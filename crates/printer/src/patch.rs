use std::fs;
use std::io::{self, Write};
use std::os::unix::prelude::OsStrExt;
use std::path::Path;
use std::time::Instant;

use grep_matcher::{Matcher, Match};
use grep_searcher::{Searcher, Sink, SinkContextKind, SinkMatch, SinkContext, SinkFinish};

use crate::counter::CounterWriter;
use crate::util::{find_iter_at_in_context, Replacer};

const ORIG_PREFIX: &[u8] = b"--- ";
const MOD_PREFIX: &[u8] = b"+++ ";

#[derive(Debug, Clone)]
enum PatchStyle {
    // The Unified format (originally GNU-only)
    Unified,
    /* TODO: determine if it's useful to support these formats
    Posix, // <- what should this be named? the 'classic' patch format
    Context,
    Ed,
    */
}

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
#[derive(Clone, Debug)]
pub struct PatchBuilder {
    config: Config,
    replacement_set: bool,
}

impl PatchBuilder {
    /// Return a new builder for configuring the patch printer.
    pub fn new() -> PatchBuilder {
        PatchBuilder { config: Config::default(), replacement_set: false, }
    }

    /// Create a Patch printer that writes results to the given writer.
    pub fn build<W: io::Write>(&self, wtr: W) -> Result<Patch<W>, io::Error> {
        if !self.replacement_set {
            return Err(io::Error::new(io::ErrorKind::Other, "replacement text not set"))
        }
        Ok(Patch {
            config: self.config.clone(),
            wtr: CounterWriter::new(wtr),
            matches: vec![],
        })
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
        self.replacement_set = true;
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

    /// Returns true if this printer should quit.
    ///
    /// This implements the logic for handling quitting after seeing a certain
    /// amount of matches. In most cases, the logic is simple, but we must
    /// permit all "after" contextual lines to print after reaching the limit.
    fn should_quit(&self) -> bool {
        false
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
        // XXX need to select this based on config style
        write_header(&mut self.patch.wtr, self.path)?;
        self.begin_printed = true;
        Ok(())
    }
}

fn write_header<W: io::Write>(mut wtr: W, path: &Path) -> io::Result<()> {
    // XXX for this, should the 'file2' path be different from 'file1'?
    let path_bytes = path.as_os_str().as_bytes();
    wtr.write(ORIG_PREFIX)?;
    wtr.write(path_bytes)?;
    wtr.write(b", ")?;
    // XXX need to get file modification date; Posix specifies it must be a
    // timestamp with this format, but... does that actually matter?
    // fs::metadata(self.path)?.modified()?
    wtr.write(b"2002-02-21 23:30:39.942229878 -0800")?;
    // XXX also: should the line-endings for patch files match the native line-endings?
    wtr.write(&[b'\n'])?;
    wtr.write(MOD_PREFIX)?;
    wtr.write(path_bytes)?;
    wtr.write(b", ")?;
    // XXX ...does the 'file2' timestamp matter?
    wtr.write(b"2002-02-21 23:30:39.942229878 -0800")?;
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

        unimplemented!();
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
        if searcher.invert_match() {
            self.record_matches(searcher, ctx.bytes(), 0..ctx.bytes().len())?;
            self.replace(searcher, ctx.bytes(), 0..ctx.bytes().len())?;
        }
        if searcher.binary_detection().convert_byte().is_some() {
            if self.binary_byte_offset.is_some() {
                return Ok(false);
            }
        }

        unimplemented!();
    }

    fn context_break(
        &mut self,
        searcher: &Searcher,
    ) -> Result<bool, io::Error> {
        // StandardImpl::new(searcher, self).write_context_separator()?;
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
        self.write_patch_header()?;
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

        unimplemented!()
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
            path: path.as_ref(),
            start_time: Instant::now(),
            match_count: 0,
            after_context_remaining: 0,
            binary_byte_offset: None,
            begin_printed: false,
        }
    }


}