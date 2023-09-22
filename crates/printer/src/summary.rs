use std::{
    cell::RefCell,
    io::{self, Write},
    path::Path,
    sync::Arc,
    time::Instant,
};

use {
    grep_matcher::Matcher,
    grep_searcher::{Searcher, Sink, SinkError, SinkFinish, SinkMatch},
    termcolor::{ColorSpec, NoColor, WriteColor},
};

use crate::{
    color::ColorSpecs,
    counter::CounterWriter,
    hyperlink::{self, HyperlinkConfig},
    stats::Stats,
    util::{find_iter_at_in_context, PrinterPath},
};

/// The configuration for the summary printer.
///
/// This is manipulated by the SummaryBuilder and then referenced by the actual
/// implementation. Once a printer is build, the configuration is frozen and
/// cannot changed.
#[derive(Debug, Clone)]
struct Config {
    kind: SummaryKind,
    colors: ColorSpecs,
    hyperlink: HyperlinkConfig,
    stats: bool,
    path: bool,
    max_matches: Option<u64>,
    exclude_zero: bool,
    separator_field: Arc<Vec<u8>>,
    separator_path: Option<u8>,
    path_terminator: Option<u8>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            kind: SummaryKind::Count,
            colors: ColorSpecs::default(),
            hyperlink: HyperlinkConfig::default(),
            stats: false,
            path: true,
            max_matches: None,
            exclude_zero: true,
            separator_field: Arc::new(b":".to_vec()),
            separator_path: None,
            path_terminator: None,
        }
    }
}

/// The type of summary output (if any) to print.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SummaryKind {
    /// Show only a count of the total number of matches (counting each line
    /// at most once) found.
    ///
    /// If the `path` setting is enabled, then the count is prefixed by the
    /// corresponding file path.
    Count,
    /// Show only a count of the total number of matches (counting possibly
    /// many matches on each line) found.
    ///
    /// If the `path` setting is enabled, then the count is prefixed by the
    /// corresponding file path.
    CountMatches,
    /// Show only the file path if and only if a match was found.
    ///
    /// This ignores the `path` setting and always shows the file path. If no
    /// file path is provided, then searching will immediately stop and return
    /// an error.
    PathWithMatch,
    /// Show only the file path if and only if a match was found.
    ///
    /// This ignores the `path` setting and always shows the file path. If no
    /// file path is provided, then searching will immediately stop and return
    /// an error.
    PathWithoutMatch,
    /// Don't show any output and the stop the search once a match is found.
    ///
    /// Note that if `stats` is enabled, then searching continues in order to
    /// compute statistics.
    Quiet,
}

impl SummaryKind {
    /// Returns true if and only if this output mode requires a file path.
    ///
    /// When an output mode requires a file path, then the summary printer
    /// will report an error at the start of every search that lacks a file
    /// path.
    fn requires_path(&self) -> bool {
        use self::SummaryKind::*;

        match *self {
            PathWithMatch | PathWithoutMatch => true,
            Count | CountMatches | Quiet => false,
        }
    }

    /// Returns true if and only if this output mode requires computing
    /// statistics, regardless of whether they have been enabled or not.
    fn requires_stats(&self) -> bool {
        use self::SummaryKind::*;

        match *self {
            CountMatches => true,
            Count | PathWithMatch | PathWithoutMatch | Quiet => false,
        }
    }

    /// Returns true if and only if a printer using this output mode can
    /// quit after seeing the first match.
    fn quit_early(&self) -> bool {
        use self::SummaryKind::*;

        match *self {
            PathWithMatch | Quiet => true,
            Count | CountMatches | PathWithoutMatch => false,
        }
    }
}

/// A builder for summary printer.
///
/// The builder permits configuring how the printer behaves. The summary
/// printer has fewer configuration options than the standard printer because
/// it aims to produce aggregate output about a single search (typically just
/// one line) instead of output for each match.
///
/// Once a `Summary` printer is built, its configuration cannot be changed.
#[derive(Clone, Debug)]
pub struct SummaryBuilder {
    config: Config,
}

impl SummaryBuilder {
    /// Return a new builder for configuring the summary printer.
    pub fn new() -> SummaryBuilder {
        SummaryBuilder { config: Config::default() }
    }

    /// Build a printer using any implementation of `termcolor::WriteColor`.
    ///
    /// The implementation of `WriteColor` used here controls whether colors
    /// are used or not when colors have been configured using the
    /// `color_specs` method.
    ///
    /// For maximum portability, callers should generally use either
    /// `termcolor::StandardStream` or `termcolor::BufferedStandardStream`
    /// where appropriate, which will automatically enable colors on Windows
    /// when possible.
    ///
    /// However, callers may also provide an arbitrary writer using the
    /// `termcolor::Ansi` or `termcolor::NoColor` wrappers, which always enable
    /// colors via ANSI escapes or always disable colors, respectively.
    ///
    /// As a convenience, callers may use `build_no_color` to automatically
    /// select the `termcolor::NoColor` wrapper to avoid needing to import
    /// from `termcolor` explicitly.
    pub fn build<W: WriteColor>(&self, wtr: W) -> Summary<W> {
        Summary {
            config: self.config.clone(),
            wtr: RefCell::new(CounterWriter::new(wtr)),
        }
    }

    /// Build a printer from any implementation of `io::Write` and never emit
    /// any colors, regardless of the user color specification settings.
    ///
    /// This is a convenience routine for
    /// `SummaryBuilder::build(termcolor::NoColor::new(wtr))`.
    pub fn build_no_color<W: io::Write>(&self, wtr: W) -> Summary<NoColor<W>> {
        self.build(NoColor::new(wtr))
    }

    /// Set the output mode for this printer.
    ///
    /// The output mode controls how aggregate results of a search are printed.
    ///
    /// By default, this printer uses the `Count` mode.
    pub fn kind(&mut self, kind: SummaryKind) -> &mut SummaryBuilder {
        self.config.kind = kind;
        self
    }

    /// Set the user color specifications to use for coloring in this printer.
    ///
    /// A [`UserColorSpec`](struct.UserColorSpec.html) can be constructed from
    /// a string in accordance with the color specification format. See the
    /// `UserColorSpec` type documentation for more details on the format.
    /// A [`ColorSpecs`](struct.ColorSpecs.html) can then be generated from
    /// zero or more `UserColorSpec`s.
    ///
    /// Regardless of the color specifications provided here, whether color
    /// is actually used or not is determined by the implementation of
    /// `WriteColor` provided to `build`. For example, if `termcolor::NoColor`
    /// is provided to `build`, then no color will ever be printed regardless
    /// of the color specifications provided here.
    ///
    /// This completely overrides any previous color specifications. This does
    /// not add to any previously provided color specifications on this
    /// builder.
    ///
    /// The default color specifications provide no styling.
    pub fn color_specs(&mut self, specs: ColorSpecs) -> &mut SummaryBuilder {
        self.config.colors = specs;
        self
    }

    /// Set the configuration to use for hyperlinks output by this printer.
    ///
    /// Regardless of the hyperlink format provided here, whether hyperlinks
    /// are actually used or not is determined by the implementation of
    /// `WriteColor` provided to `build`. For example, if `termcolor::NoColor`
    /// is provided to `build`, then no hyperlinks will ever be printed
    /// regardless of the format provided here.
    ///
    /// This completely overrides any previous hyperlink format.
    ///
    /// The default configuration results in not emitting any hyperlinks.
    pub fn hyperlink(
        &mut self,
        config: HyperlinkConfig,
    ) -> &mut SummaryBuilder {
        self.config.hyperlink = config;
        self
    }

    /// Enable the gathering of various aggregate statistics.
    ///
    /// When this is enabled (it's disabled by default), statistics will be
    /// gathered for all uses of `Summary` printer returned by `build`,
    /// including but not limited to, the total number of matches, the total
    /// number of bytes searched and the total number of bytes printed.
    ///
    /// Aggregate statistics can be accessed via the sink's
    /// [`SummarySink::stats`](struct.SummarySink.html#method.stats)
    /// method.
    ///
    /// When this is enabled, this printer may need to do extra work in order
    /// to compute certain statistics, which could cause the search to take
    /// longer. For example, in `Quiet` mode, a search can quit after finding
    /// the first match, but if `stats` is enabled, then the search will
    /// continue after the first match in order to compute statistics.
    ///
    /// For a complete description of available statistics, see
    /// [`Stats`](struct.Stats.html).
    ///
    /// Note that some output modes, such as `CountMatches`, automatically
    /// enable this option even if it has been explicitly disabled.
    pub fn stats(&mut self, yes: bool) -> &mut SummaryBuilder {
        self.config.stats = yes;
        self
    }

    /// When enabled, if a path was given to the printer, then it is shown in
    /// the output (either as a heading or as a prefix to each matching line).
    /// When disabled, then no paths are ever included in the output even when
    /// a path is provided to the printer.
    ///
    /// This setting has no effect in `PathWithMatch` and `PathWithoutMatch`
    /// modes.
    ///
    /// This is enabled by default.
    pub fn path(&mut self, yes: bool) -> &mut SummaryBuilder {
        self.config.path = yes;
        self
    }

    /// Set the maximum amount of matches that are printed.
    ///
    /// If multi line search is enabled and a match spans multiple lines, then
    /// that match is counted exactly once for the purposes of enforcing this
    /// limit, regardless of how many lines it spans.
    ///
    /// This is disabled by default.
    pub fn max_matches(&mut self, limit: Option<u64>) -> &mut SummaryBuilder {
        self.config.max_matches = limit;
        self
    }

    /// Exclude count-related summary results with no matches.
    ///
    /// When enabled and the mode is either `Count` or `CountMatches`, then
    /// results are not printed if no matches were found. Otherwise, every
    /// search prints a result with a possibly `0` number of matches.
    ///
    /// This is enabled by default.
    pub fn exclude_zero(&mut self, yes: bool) -> &mut SummaryBuilder {
        self.config.exclude_zero = yes;
        self
    }

    /// Set the separator used between fields for the `Count` and
    /// `CountMatches` modes.
    ///
    /// By default, this is set to `:`.
    pub fn separator_field(&mut self, sep: Vec<u8>) -> &mut SummaryBuilder {
        self.config.separator_field = Arc::new(sep);
        self
    }

    /// Set the path separator used when printing file paths.
    ///
    /// Typically, printing is done by emitting the file path as is. However,
    /// this setting provides the ability to use a different path separator
    /// from what the current environment has configured.
    ///
    /// A typical use for this option is to permit cygwin users on Windows to
    /// set the path separator to `/` instead of using the system default of
    /// `\`.
    ///
    /// This is disabled by default.
    pub fn separator_path(&mut self, sep: Option<u8>) -> &mut SummaryBuilder {
        self.config.separator_path = sep;
        self
    }

    /// Set the path terminator used.
    ///
    /// The path terminator is a byte that is printed after every file path
    /// emitted by this printer.
    ///
    /// If no path terminator is set (the default), then paths are terminated
    /// by either new lines or the configured field separator.
    pub fn path_terminator(
        &mut self,
        terminator: Option<u8>,
    ) -> &mut SummaryBuilder {
        self.config.path_terminator = terminator;
        self
    }
}

/// The summary printer, which emits aggregate results from a search.
///
/// Aggregate results generally correspond to file paths and/or the number of
/// matches found.
///
/// A default printer can be created with either of the `Summary::new` or
/// `Summary::new_no_color` constructors. However, there are a number of
/// options that configure this printer's output. Those options can be
/// configured using [`SummaryBuilder`](struct.SummaryBuilder.html).
///
/// This type is generic over `W`, which represents any implementation of
/// the `termcolor::WriteColor` trait.
#[derive(Debug)]
pub struct Summary<W> {
    config: Config,
    wtr: RefCell<CounterWriter<W>>,
}

impl<W: WriteColor> Summary<W> {
    /// Return a summary printer with a default configuration that writes
    /// matches to the given writer.
    ///
    /// The writer should be an implementation of `termcolor::WriteColor`
    /// and not just a bare implementation of `io::Write`. To use a normal
    /// `io::Write` implementation (simultaneously sacrificing colors), use
    /// the `new_no_color` constructor.
    ///
    /// The default configuration uses the `Count` summary mode.
    pub fn new(wtr: W) -> Summary<W> {
        SummaryBuilder::new().build(wtr)
    }
}

impl<W: io::Write> Summary<NoColor<W>> {
    /// Return a summary printer with a default configuration that writes
    /// matches to the given writer.
    ///
    /// The writer can be any implementation of `io::Write`. With this
    /// constructor, the printer will never emit colors.
    ///
    /// The default configuration uses the `Count` summary mode.
    pub fn new_no_color(wtr: W) -> Summary<NoColor<W>> {
        SummaryBuilder::new().build_no_color(wtr)
    }
}

impl<W: WriteColor> Summary<W> {
    /// Return an implementation of `Sink` for the summary printer.
    ///
    /// This does not associate the printer with a file path, which means this
    /// implementation will never print a file path. If the output mode of
    /// this summary printer does not make sense without a file path (such as
    /// `PathWithMatch` or `PathWithoutMatch`), then any searches executed
    /// using this sink will immediately quit with an error.
    pub fn sink<'s, M: Matcher>(
        &'s mut self,
        matcher: M,
    ) -> SummarySink<'static, 's, M, W> {
        let interpolator =
            hyperlink::Interpolator::new(&self.config.hyperlink);
        let stats = if self.config.stats || self.config.kind.requires_stats() {
            Some(Stats::new())
        } else {
            None
        };
        SummarySink {
            matcher,
            summary: self,
            interpolator,
            path: None,
            start_time: Instant::now(),
            match_count: 0,
            binary_byte_offset: None,
            stats,
        }
    }

    /// Return an implementation of `Sink` associated with a file path.
    ///
    /// When the printer is associated with a path, then it may, depending on
    /// its configuration, print the path.
    pub fn sink_with_path<'p, 's, M, P>(
        &'s mut self,
        matcher: M,
        path: &'p P,
    ) -> SummarySink<'p, 's, M, W>
    where
        M: Matcher,
        P: ?Sized + AsRef<Path>,
    {
        if !self.config.path && !self.config.kind.requires_path() {
            return self.sink(matcher);
        }
        let interpolator =
            hyperlink::Interpolator::new(&self.config.hyperlink);
        let stats = if self.config.stats || self.config.kind.requires_stats() {
            Some(Stats::new())
        } else {
            None
        };
        let ppath = PrinterPath::new(path.as_ref())
            .with_separator(self.config.separator_path);
        SummarySink {
            matcher,
            summary: self,
            interpolator,
            path: Some(ppath),
            start_time: Instant::now(),
            match_count: 0,
            binary_byte_offset: None,
            stats,
        }
    }
}

impl<W> Summary<W> {
    /// Returns true if and only if this printer has written at least one byte
    /// to the underlying writer during any of the previous searches.
    pub fn has_written(&self) -> bool {
        self.wtr.borrow().total_count() > 0
    }

    /// Return a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.wtr.get_mut().get_mut()
    }

    /// Consume this printer and return back ownership of the underlying
    /// writer.
    pub fn into_inner(self) -> W {
        self.wtr.into_inner().into_inner()
    }
}

/// An implementation of `Sink` associated with a matcher and an optional file
/// path for the summary printer.
///
/// This type is generic over a few type parameters:
///
/// * `'p` refers to the lifetime of the file path, if one is provided. When
///   no file path is given, then this is `'static`.
/// * `'s` refers to the lifetime of the
///   [`Summary`](struct.Summary.html)
///   printer that this type borrows.
/// * `M` refers to the type of matcher used by
///   `grep_searcher::Searcher` that is reporting results to this sink.
/// * `W` refers to the underlying writer that this printer is writing its
///   output to.
#[derive(Debug)]
pub struct SummarySink<'p, 's, M: Matcher, W> {
    matcher: M,
    summary: &'s mut Summary<W>,
    interpolator: hyperlink::Interpolator,
    path: Option<PrinterPath<'p>>,
    start_time: Instant,
    match_count: u64,
    binary_byte_offset: Option<u64>,
    stats: Option<Stats>,
}

impl<'p, 's, M: Matcher, W: WriteColor> SummarySink<'p, 's, M, W> {
    /// Returns true if and only if this printer received a match in the
    /// previous search.
    ///
    /// This is unaffected by the result of searches before the previous
    /// search.
    pub fn has_match(&self) -> bool {
        match self.summary.config.kind {
            SummaryKind::PathWithoutMatch => self.match_count == 0,
            _ => self.match_count > 0,
        }
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
    ///
    /// This only returns stats if they were requested via the
    /// [`SummaryBuilder`](struct.SummaryBuilder.html)
    /// configuration.
    pub fn stats(&self) -> Option<&Stats> {
        self.stats.as_ref()
    }

    /// Returns true if and only if the searcher may report matches over
    /// multiple lines.
    ///
    /// Note that this doesn't just return whether the searcher is in multi
    /// line mode, but also checks if the matter can match over multiple lines.
    /// If it can't, then we don't need multi line handling, even if the
    /// searcher has multi line mode enabled.
    fn multi_line(&self, searcher: &Searcher) -> bool {
        searcher.multi_line_with_matcher(&self.matcher)
    }

    /// Returns true if this printer should quit.
    ///
    /// This implements the logic for handling quitting after seeing a certain
    /// amount of matches. In most cases, the logic is simple, but we must
    /// permit all "after" contextual lines to print after reaching the limit.
    fn should_quit(&self) -> bool {
        let limit = match self.summary.config.max_matches {
            None => return false,
            Some(limit) => limit,
        };
        self.match_count >= limit
    }

    /// If this printer has a file path associated with it, then this will
    /// write that path to the underlying writer followed by a line terminator.
    /// (If a path terminator is set, then that is used instead of the line
    /// terminator.)
    fn write_path_line(&mut self, searcher: &Searcher) -> io::Result<()> {
        if self.path.is_some() {
            self.write_path()?;
            if let Some(term) = self.summary.config.path_terminator {
                self.write(&[term])?;
            } else {
                self.write_line_term(searcher)?;
            }
        }
        Ok(())
    }

    /// If this printer has a file path associated with it, then this will
    /// write that path to the underlying writer followed by the field
    /// separator. (If a path terminator is set, then that is used instead of
    /// the field separator.)
    fn write_path_field(&mut self) -> io::Result<()> {
        if self.path.is_some() {
            self.write_path()?;
            if let Some(term) = self.summary.config.path_terminator {
                self.write(&[term])?;
            } else {
                self.write(&self.summary.config.separator_field)?;
            }
        }
        Ok(())
    }

    /// If this printer has a file path associated with it, then this will
    /// write that path to the underlying writer in the appropriate style
    /// (color and hyperlink).
    fn write_path(&mut self) -> io::Result<()> {
        if self.path.is_some() {
            let status = self.start_hyperlink()?;
            self.write_spec(
                self.summary.config.colors.path(),
                self.path.as_ref().unwrap().as_bytes(),
            )?;
            self.end_hyperlink(status)?;
        }
        Ok(())
    }

    /// Starts a hyperlink span when applicable.
    fn start_hyperlink(
        &mut self,
    ) -> io::Result<hyperlink::InterpolatorStatus> {
        let Some(hyperpath) =
            self.path.as_ref().and_then(|p| p.as_hyperlink())
        else {
            return Ok(hyperlink::InterpolatorStatus::inactive());
        };
        let values = hyperlink::Values::new(hyperpath);
        self.interpolator.begin(&values, &mut *self.summary.wtr.borrow_mut())
    }

    fn end_hyperlink(
        &self,
        status: hyperlink::InterpolatorStatus,
    ) -> io::Result<()> {
        self.interpolator.finish(status, &mut *self.summary.wtr.borrow_mut())
    }

    /// Write the line terminator configured on the given searcher.
    fn write_line_term(&self, searcher: &Searcher) -> io::Result<()> {
        self.write(searcher.line_terminator().as_bytes())
    }

    /// Write the given bytes using the give style.
    fn write_spec(&self, spec: &ColorSpec, buf: &[u8]) -> io::Result<()> {
        self.summary.wtr.borrow_mut().set_color(spec)?;
        self.write(buf)?;
        self.summary.wtr.borrow_mut().reset()?;
        Ok(())
    }

    /// Write all of the given bytes.
    fn write(&self, buf: &[u8]) -> io::Result<()> {
        self.summary.wtr.borrow_mut().write_all(buf)
    }
}

impl<'p, 's, M: Matcher, W: WriteColor> Sink for SummarySink<'p, 's, M, W> {
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        let is_multi_line = self.multi_line(searcher);
        let sink_match_count = if self.stats.is_none() && !is_multi_line {
            1
        } else {
            // This gives us as many bytes as the searcher can offer. This
            // isn't guaranteed to hold the necessary context to get match
            // detection correct (because of look-around), but it does in
            // practice.
            let buf = mat.buffer();
            let range = mat.bytes_range_in_buffer();
            let mut count = 0;
            find_iter_at_in_context(
                searcher,
                &self.matcher,
                buf,
                range,
                |_| {
                    count += 1;
                    true
                },
            )?;
            count
        };
        if is_multi_line {
            self.match_count += sink_match_count;
        } else {
            self.match_count += 1;
        }
        if let Some(ref mut stats) = self.stats {
            stats.add_matches(sink_match_count);
            stats.add_matched_lines(mat.lines().count() as u64);
        } else if self.summary.config.kind.quit_early() {
            return Ok(false);
        }
        Ok(!self.should_quit())
    }

    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, io::Error> {
        if self.path.is_none() && self.summary.config.kind.requires_path() {
            return Err(io::Error::error_message(format!(
                "output kind {:?} requires a file path",
                self.summary.config.kind,
            )));
        }
        self.summary.wtr.borrow_mut().reset_count();
        self.start_time = Instant::now();
        self.match_count = 0;
        self.binary_byte_offset = None;
        if self.summary.config.max_matches == Some(0) {
            return Ok(false);
        }

        Ok(true)
    }

    fn finish(
        &mut self,
        searcher: &Searcher,
        finish: &SinkFinish,
    ) -> Result<(), io::Error> {
        self.binary_byte_offset = finish.binary_byte_offset();
        if let Some(ref mut stats) = self.stats {
            stats.add_elapsed(self.start_time.elapsed());
            stats.add_searches(1);
            if self.match_count > 0 {
                stats.add_searches_with_match(1);
            }
            stats.add_bytes_searched(finish.byte_count());
            stats.add_bytes_printed(self.summary.wtr.borrow().count());
        }
        // If our binary detection method says to quit after seeing binary
        // data, then we shouldn't print any results at all, even if we've
        // found a match before detecting binary data. The intent here is to
        // keep BinaryDetection::quit as a form of filter. Otherwise, we can
        // present a matching file with a smaller number of matches than
        // there might be, which can be quite misleading.
        //
        // If our binary detection method is to convert binary data, then we
        // don't quit and therefore search the entire contents of the file.
        //
        // There is an unfortunate inconsistency here. Namely, when using
        // Quiet or PathWithMatch, then the printer can quit after the first
        // match seen, which could be long before seeing binary data. This
        // means that using PathWithMatch can print a path where as using
        // Count might not print it at all because of binary data.
        //
        // It's not possible to fix this without also potentially significantly
        // impacting the performance of Quiet or PathWithMatch, so we accept
        // the bug.
        if self.binary_byte_offset.is_some()
            && searcher.binary_detection().quit_byte().is_some()
        {
            // Squash the match count. The statistics reported will still
            // contain the match count, but the "official" match count should
            // be zero.
            self.match_count = 0;
            return Ok(());
        }

        let show_count =
            !self.summary.config.exclude_zero || self.match_count > 0;
        match self.summary.config.kind {
            SummaryKind::Count => {
                if show_count {
                    self.write_path_field()?;
                    self.write(self.match_count.to_string().as_bytes())?;
                    self.write_line_term(searcher)?;
                }
            }
            SummaryKind::CountMatches => {
                if show_count {
                    self.write_path_field()?;
                    let stats = self
                        .stats
                        .as_ref()
                        .expect("CountMatches should enable stats tracking");
                    self.write(stats.matches().to_string().as_bytes())?;
                    self.write_line_term(searcher)?;
                }
            }
            SummaryKind::PathWithMatch => {
                if self.match_count > 0 {
                    self.write_path_line(searcher)?;
                }
            }
            SummaryKind::PathWithoutMatch => {
                if self.match_count == 0 {
                    self.write_path_line(searcher)?;
                }
            }
            SummaryKind::Quiet => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use grep_regex::RegexMatcher;
    use grep_searcher::SearcherBuilder;
    use termcolor::NoColor;

    use super::{Summary, SummaryBuilder, SummaryKind};

    const SHERLOCK: &'static [u8] = b"\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
can extract a clew from a wisp of straw or a flake of cigar ash;
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.
";

    fn printer_contents(printer: &mut Summary<NoColor<Vec<u8>>>) -> String {
        String::from_utf8(printer.get_mut().get_ref().to_owned()).unwrap()
    }

    #[test]
    fn path_with_match_error() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::PathWithMatch)
            .build_no_color(vec![]);
        let res = SearcherBuilder::new().build().search_reader(
            &matcher,
            SHERLOCK,
            printer.sink(&matcher),
        );
        assert!(res.is_err());
    }

    #[test]
    fn path_without_match_error() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::PathWithoutMatch)
            .build_no_color(vec![]);
        let res = SearcherBuilder::new().build().search_reader(
            &matcher,
            SHERLOCK,
            printer.sink(&matcher),
        );
        assert!(res.is_err());
    }

    #[test]
    fn count_no_path() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(&matcher, SHERLOCK, printer.sink(&matcher))
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("2\n", got);
    }

    #[test]
    fn count_no_path_even_with_path() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .path(false)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("2\n", got);
    }

    #[test]
    fn count_path() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("sherlock:2\n", got);
    }

    #[test]
    fn count_path_with_zero() {
        let matcher = RegexMatcher::new(r"NO MATCH").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .exclude_zero(false)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("sherlock:0\n", got);
    }

    #[test]
    fn count_path_without_zero() {
        let matcher = RegexMatcher::new(r"NO MATCH").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .exclude_zero(true)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("", got);
    }

    #[test]
    fn count_path_field_separator() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .separator_field(b"ZZ".to_vec())
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("sherlockZZ2\n", got);
    }

    #[test]
    fn count_path_terminator() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .path_terminator(Some(b'\x00'))
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("sherlock\x002\n", got);
    }

    #[test]
    fn count_path_separator() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .separator_path(Some(b'\\'))
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "/home/andrew/sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("\\home\\andrew\\sherlock:2\n", got);
    }

    #[test]
    fn count_max_matches() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Count)
            .max_matches(Some(1))
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(&matcher, SHERLOCK, printer.sink(&matcher))
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("1\n", got);
    }

    #[test]
    fn count_matches() {
        let matcher = RegexMatcher::new(r"Watson|Sherlock").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::CountMatches)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("sherlock:4\n", got);
    }

    #[test]
    fn path_with_match_found() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::PathWithMatch)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("sherlock\n", got);
    }

    #[test]
    fn path_with_match_not_found() {
        let matcher = RegexMatcher::new(r"ZZZZZZZZ").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::PathWithMatch)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("", got);
    }

    #[test]
    fn path_without_match_found() {
        let matcher = RegexMatcher::new(r"ZZZZZZZZZ").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::PathWithoutMatch)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("sherlock\n", got);
    }

    #[test]
    fn path_without_match_not_found() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::PathWithoutMatch)
            .build_no_color(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(
                &matcher,
                SHERLOCK,
                printer.sink_with_path(&matcher, "sherlock"),
            )
            .unwrap();

        let got = printer_contents(&mut printer);
        assert_eq_printed!("", got);
    }

    #[test]
    fn quiet() {
        let matcher = RegexMatcher::new(r"Watson|Sherlock").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Quiet)
            .build_no_color(vec![]);
        let match_count = {
            let mut sink = printer.sink_with_path(&matcher, "sherlock");
            SearcherBuilder::new()
                .build()
                .search_reader(&matcher, SHERLOCK, &mut sink)
                .unwrap();
            sink.match_count
        };

        let got = printer_contents(&mut printer);
        assert_eq_printed!("", got);
        // There is actually more than one match, but Quiet should quit after
        // finding the first one.
        assert_eq!(1, match_count);
    }

    #[test]
    fn quiet_with_stats() {
        let matcher = RegexMatcher::new(r"Watson|Sherlock").unwrap();
        let mut printer = SummaryBuilder::new()
            .kind(SummaryKind::Quiet)
            .stats(true)
            .build_no_color(vec![]);
        let match_count = {
            let mut sink = printer.sink_with_path(&matcher, "sherlock");
            SearcherBuilder::new()
                .build()
                .search_reader(&matcher, SHERLOCK, &mut sink)
                .unwrap();
            sink.match_count
        };

        let got = printer_contents(&mut printer);
        assert_eq_printed!("", got);
        // There is actually more than one match, and Quiet will usually quit
        // after finding the first one, but since we request stats, it will
        // mush on to find all matches.
        assert_eq!(3, match_count);
    }
}
