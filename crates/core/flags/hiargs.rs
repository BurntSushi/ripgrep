/*!
Provides the definition of high level arguments from CLI flags.
*/

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use {
    bstr::BString,
    grep::printer::{ColorSpecs, SummaryKind},
};

use crate::{
    flags::lowargs::{
        BinaryMode, BoundaryMode, BufferMode, CaseMode, ColorChoice,
        ContextMode, ContextSeparator, EncodingMode, EngineChoice,
        FieldContextSeparator, FieldMatchSeparator, LowArgs, MmapMode, Mode,
        PatternSource, SearchMode, SortMode, SortModeKind, TypeChange,
    },
    haystack::{Haystack, HaystackBuilder},
    search::{PatternMatcher, Printer, SearchWorker, SearchWorkerBuilder},
};

/// A high level representation of CLI arguments.
///
/// The distinction between low and high level arguments is somewhat arbitrary
/// and wishy washy. The main idea here is that high level arguments generally
/// require all of CLI parsing to be finished. For example, one cannot
/// construct a glob matcher until all of the glob patterns are known.
///
/// So while low level arguments are collected during parsing itself, high
/// level arguments aren't created until parsing has completely finished.
#[derive(Debug)]
pub(crate) struct HiArgs {
    binary: BinaryDetection,
    boundary: Option<BoundaryMode>,
    buffer: BufferMode,
    byte_offset: bool,
    case: CaseMode,
    color: ColorChoice,
    colors: grep::printer::ColorSpecs,
    column: bool,
    context: ContextMode,
    context_separator: ContextSeparator,
    crlf: bool,
    dfa_size_limit: Option<usize>,
    encoding: EncodingMode,
    engine: EngineChoice,
    field_context_separator: FieldContextSeparator,
    field_match_separator: FieldMatchSeparator,
    file_separator: Option<Vec<u8>>,
    fixed_strings: bool,
    follow: bool,
    globs: ignore::overrides::Override,
    heading: bool,
    hidden: bool,
    hyperlink_config: grep::printer::HyperlinkConfig,
    ignore_file_case_insensitive: bool,
    ignore_file: Vec<PathBuf>,
    include_zero: bool,
    invert_match: bool,
    is_terminal_stdout: bool,
    line_number: bool,
    max_columns: Option<u64>,
    max_columns_preview: bool,
    max_count: Option<u64>,
    max_depth: Option<usize>,
    max_filesize: Option<u64>,
    mmap_choice: grep::searcher::MmapChoice,
    mode: Mode,
    multiline: bool,
    multiline_dotall: bool,
    no_ignore_dot: bool,
    no_ignore_exclude: bool,
    no_ignore_files: bool,
    no_ignore_global: bool,
    no_ignore_parent: bool,
    no_ignore_vcs: bool,
    no_require_git: bool,
    no_unicode: bool,
    null_data: bool,
    one_file_system: bool,
    only_matching: bool,
    path_separator: Option<u8>,
    paths: Paths,
    path_terminator: Option<u8>,
    patterns: Patterns,
    pre: Option<PathBuf>,
    pre_globs: ignore::overrides::Override,
    quiet: bool,
    quit_after_match: bool,
    regex_size_limit: Option<usize>,
    replace: Option<BString>,
    search_zip: bool,
    sort: Option<SortMode>,
    stats: Option<grep::printer::Stats>,
    stop_on_nonmatch: bool,
    threads: usize,
    trim: bool,
    types: ignore::types::Types,
    vimgrep: bool,
    with_filename: bool,
}

impl HiArgs {
    /// Convert low level arguments into high level arguments.
    ///
    /// This process can fail for a variety of reasons. For example, invalid
    /// globs or some kind of environment issue.
    pub(crate) fn from_low_args(mut low: LowArgs) -> anyhow::Result<HiArgs> {
        // Callers should not be trying to convert low-level arguments when
        // a short-circuiting special mode is present.
        assert_eq!(None, low.special, "special mode demands short-circuiting");
        // If the sorting mode isn't supported, then we bail loudly. I'm not
        // sure if this is the right thing to do. We could silently "not sort"
        // as well. If we wanted to go that route, then we could just set
        // `low.sort = None` if `supported()` returns an error.
        if let Some(ref sort) = low.sort {
            sort.supported()?;
        }

        // We modify the mode in-place on `low` so that subsequent conversions
        // see the correct mode.
        match low.mode {
            Mode::Search(ref mut mode) => match *mode {
                // treat `-v --count-matches` as `-v --count`
                SearchMode::CountMatches if low.invert_match => {
                    *mode = SearchMode::Count;
                }
                // treat `-o --count` as `--count-matches`
                SearchMode::Count if low.only_matching => {
                    *mode = SearchMode::CountMatches;
                }
                _ => {}
            },
            _ => {}
        }

        let mut state = State::new()?;
        let patterns = Patterns::from_low_args(&mut state, &mut low)?;
        let paths = Paths::from_low_args(&mut state, &patterns, &mut low)?;

        let binary = BinaryDetection::from_low_args(&state, &low);
        let colors = take_color_specs(&mut state, &mut low);
        let hyperlink_config = take_hyperlink_config(&mut state, &mut low)?;
        let stats = stats(&low);
        let types = types(&low)?;
        let globs = globs(&state, &low)?;
        let pre_globs = preprocessor_globs(&state, &low)?;

        let color = match low.color {
            ColorChoice::Auto if !state.is_terminal_stdout => {
                ColorChoice::Never
            }
            _ => low.color,
        };
        let column = low.column.unwrap_or(low.vimgrep);
        let heading = match low.heading {
            None => !low.vimgrep && state.is_terminal_stdout,
            Some(false) => false,
            Some(true) => !low.vimgrep,
        };
        let path_terminator = if low.null { Some(b'\x00') } else { None };
        let quit_after_match = stats.is_none() && low.quiet;
        let threads = if low.sort.is_some() || paths.is_one_file {
            1
        } else if let Some(threads) = low.threads {
            threads
        } else {
            std::thread::available_parallelism().map_or(1, |n| n.get()).min(12)
        };
        log::debug!("using {threads} thread(s)");
        let with_filename = low
            .with_filename
            .unwrap_or_else(|| low.vimgrep || !paths.is_one_file);

        let file_separator = match low.mode {
            Mode::Search(SearchMode::Standard) => {
                if heading {
                    Some(b"".to_vec())
                } else if let ContextMode::Limited(ref limited) = low.context {
                    let (before, after) = limited.get();
                    if before > 0 || after > 0 {
                        low.context_separator.clone().into_bytes()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        let line_number = low.line_number.unwrap_or_else(|| {
            if low.quiet {
                return false;
            }
            let Mode::Search(ref search_mode) = low.mode else { return false };
            match *search_mode {
                SearchMode::FilesWithMatches
                | SearchMode::FilesWithoutMatch
                | SearchMode::Count
                | SearchMode::CountMatches => return false,
                SearchMode::JSON => return true,
                SearchMode::Standard => {
                    // A few things can imply counting line numbers. In
                    // particular, we generally want to show line numbers by
                    // default when printing to a tty for human consumption,
                    // except for one interesting case: when we're only
                    // searching stdin. This makes pipelines work as expected.
                    (state.is_terminal_stdout && !paths.is_only_stdin())
                        || column
                        || low.vimgrep
                }
            }
        });

        let mmap_choice = {
            // SAFETY: Memory maps are difficult to impossible to encapsulate
            // safely in a portable way that doesn't simultaneously negate some
            // of the benfits of using memory maps. For ripgrep's use, we never
            // mutate a memory map and generally never store the contents of
            // memory map in a data structure that depends on immutability.
            // Generally speaking, the worst thing that can happen is a SIGBUS
            // (if the underlying file is truncated while reading it), which
            // will cause ripgrep to abort. This reasoning should be treated as
            // suspect.
            let maybe = unsafe { grep::searcher::MmapChoice::auto() };
            let never = grep::searcher::MmapChoice::never();
            match low.mmap {
                MmapMode::Auto => {
                    if paths.paths.len() <= 10
                        && paths.paths.iter().all(|p| p.is_file())
                    {
                        // If we're only searching a few paths and all of them
                        // are files, then memory maps are probably faster.
                        maybe
                    } else {
                        never
                    }
                }
                MmapMode::AlwaysTryMmap => maybe,
                MmapMode::Never => never,
            }
        };

        Ok(HiArgs {
            mode: low.mode,
            patterns,
            paths,
            binary,
            boundary: low.boundary,
            buffer: low.buffer,
            byte_offset: low.byte_offset,
            case: low.case,
            color,
            colors,
            column,
            context: low.context,
            context_separator: low.context_separator,
            crlf: low.crlf,
            dfa_size_limit: low.dfa_size_limit,
            encoding: low.encoding,
            engine: low.engine,
            field_context_separator: low.field_context_separator,
            field_match_separator: low.field_match_separator,
            file_separator,
            fixed_strings: low.fixed_strings,
            follow: low.follow,
            heading,
            hidden: low.hidden,
            hyperlink_config,
            ignore_file: low.ignore_file,
            ignore_file_case_insensitive: low.ignore_file_case_insensitive,
            include_zero: low.include_zero,
            invert_match: low.invert_match,
            is_terminal_stdout: state.is_terminal_stdout,
            line_number,
            max_columns: low.max_columns,
            max_columns_preview: low.max_columns_preview,
            max_count: low.max_count,
            max_depth: low.max_depth,
            max_filesize: low.max_filesize,
            mmap_choice,
            multiline: low.multiline,
            multiline_dotall: low.multiline_dotall,
            no_ignore_dot: low.no_ignore_dot,
            no_ignore_exclude: low.no_ignore_exclude,
            no_ignore_files: low.no_ignore_files,
            no_ignore_global: low.no_ignore_global,
            no_ignore_parent: low.no_ignore_parent,
            no_ignore_vcs: low.no_ignore_vcs,
            no_require_git: low.no_require_git,
            no_unicode: low.no_unicode,
            null_data: low.null_data,
            one_file_system: low.one_file_system,
            only_matching: low.only_matching,
            globs,
            path_separator: low.path_separator,
            path_terminator,
            pre: low.pre,
            pre_globs,
            quiet: low.quiet,
            quit_after_match,
            regex_size_limit: low.regex_size_limit,
            replace: low.replace,
            search_zip: low.search_zip,
            sort: low.sort,
            stats,
            stop_on_nonmatch: low.stop_on_nonmatch,
            threads,
            trim: low.trim,
            types,
            vimgrep: low.vimgrep,
            with_filename,
        })
    }

    /// Returns a writer for printing buffers to stdout.
    ///
    /// This is intended to be used from multiple threads. Namely, a buffer
    /// writer can create new buffers that are sent to threads. Threads can
    /// then independently write to the buffers. Once a unit of work is
    /// complete, a buffer can be given to the buffer writer to write to
    /// stdout.
    pub(crate) fn buffer_writer(&self) -> termcolor::BufferWriter {
        let mut wtr =
            termcolor::BufferWriter::stdout(self.color.to_termcolor());
        wtr.separator(self.file_separator.clone());
        wtr
    }

    /// Returns true when ripgrep had to guess to search the current working
    /// directory. That is, it's true when ripgrep is called without any file
    /// paths or directories to search.
    ///
    /// Other than changing how file paths are printed (i.e., without the
    /// leading `./`), it's also useful to know for diagnostic reasons. For
    /// example, ripgrep will print an error message when nothing is searched
    /// since it's possible the ignore rules in play are too aggressive. But
    /// this warning is only emitted when ripgrep was called without any
    /// explicit file paths since otherwise the warning would likely be too
    /// aggressive.
    pub(crate) fn has_implicit_path(&self) -> bool {
        self.paths.has_implicit_path
    }

    /// Return a properly configured builder for constructing haystacks.
    ///
    /// The builder can be used to turn a directory entry (from the `ignore`
    /// crate) into something that can be searched.
    pub(crate) fn haystack_builder(&self) -> HaystackBuilder {
        let mut builder = HaystackBuilder::new();
        builder.strip_dot_prefix(self.paths.has_implicit_path);
        builder
    }

    /// Return the matcher that should be used for searching using the engine
    /// choice made by the user.
    ///
    /// If there was a problem building the matcher (e.g., a syntax error),
    /// then this returns an error.
    pub(crate) fn matcher(&self) -> anyhow::Result<PatternMatcher> {
        match self.engine {
            EngineChoice::Default => match self.matcher_rust() {
                Ok(m) => Ok(m),
                Err(err) => {
                    anyhow::bail!(suggest_other_engine(err.to_string()));
                }
            },
            EngineChoice::PCRE2 => Ok(self.matcher_pcre2()?),
            EngineChoice::Auto => {
                let rust_err = match self.matcher_rust() {
                    Ok(m) => return Ok(m),
                    Err(err) => err,
                };
                log::debug!(
                    "error building Rust regex in hybrid mode:\n{rust_err}",
                );

                let pcre_err = match self.matcher_pcre2() {
                    Ok(m) => return Ok(m),
                    Err(err) => err,
                };
                let divider = "~".repeat(79);
                anyhow::bail!(
                    "regex could not be compiled with either the default \
                     regex engine or with PCRE2.\n\n\
                     default regex engine error:\n\
                     {divider}\n\
                     {rust_err}\n\
                     {divider}\n\n\
                     PCRE2 regex engine error:\n{pcre_err}",
                );
            }
        }
    }

    /// Build a matcher using PCRE2.
    ///
    /// If there was a problem building the matcher (such as a regex syntax
    /// error), then an error is returned.
    ///
    /// If the `pcre2` feature is not enabled then this always returns an
    /// error.
    fn matcher_pcre2(&self) -> anyhow::Result<PatternMatcher> {
        #[cfg(feature = "pcre2")]
        {
            let mut builder = grep::pcre2::RegexMatcherBuilder::new();
            builder.multi_line(true).fixed_strings(self.fixed_strings);
            match self.case {
                CaseMode::Sensitive => builder.caseless(false),
                CaseMode::Insensitive => builder.caseless(true),
                CaseMode::Smart => builder.case_smart(true),
            };
            if let Some(ref boundary) = self.boundary {
                match *boundary {
                    BoundaryMode::Line => builder.whole_line(true),
                    BoundaryMode::Word => builder.word(true),
                };
            }
            // For whatever reason, the JIT craps out during regex compilation with
            // a "no more memory" error on 32 bit systems. So don't use it there.
            if cfg!(target_pointer_width = "64") {
                builder
                    .jit_if_available(true)
                    // The PCRE2 docs say that 32KB is the default, and that 1MB
                    // should be big enough for anything. But let's crank it to
                    // 10MB.
                    .max_jit_stack_size(Some(10 * (1 << 20)));
            }
            if !self.no_unicode {
                builder.utf(true).ucp(true);
            }
            if self.multiline {
                builder.dotall(self.multiline_dotall);
            }
            if self.crlf {
                builder.crlf(true);
            }
            let m = builder.build_many(&self.patterns.patterns)?;
            Ok(PatternMatcher::PCRE2(m))
        }
        #[cfg(not(feature = "pcre2"))]
        {
            Err(anyhow::anyhow!(
                "PCRE2 is not available in this build of ripgrep"
            ))
        }
    }

    /// Build a matcher using Rust's regex engine.
    ///
    /// If there was a problem building the matcher (such as a regex syntax
    /// error), then an error is returned.
    fn matcher_rust(&self) -> anyhow::Result<PatternMatcher> {
        let mut builder = grep::regex::RegexMatcherBuilder::new();
        builder
            .multi_line(true)
            .unicode(!self.no_unicode)
            .octal(false)
            .fixed_strings(self.fixed_strings);
        match self.case {
            CaseMode::Sensitive => builder.case_insensitive(false),
            CaseMode::Insensitive => builder.case_insensitive(true),
            CaseMode::Smart => builder.case_smart(true),
        };
        if let Some(ref boundary) = self.boundary {
            match *boundary {
                BoundaryMode::Line => builder.whole_line(true),
                BoundaryMode::Word => builder.word(true),
            };
        }
        if self.multiline {
            builder.dot_matches_new_line(self.multiline_dotall);
            if self.crlf {
                builder.crlf(true).line_terminator(None);
            }
        } else {
            builder.line_terminator(Some(b'\n')).dot_matches_new_line(false);
            if self.crlf {
                builder.crlf(true);
            }
            // We don't need to set this in multiline mode since multiline
            // matchers don't use optimizations related to line terminators.
            // Moreover, a multiline regex used with --null-data should
            // be allowed to match NUL bytes explicitly, which this would
            // otherwise forbid.
            if self.null_data {
                builder.line_terminator(Some(b'\x00'));
            }
        }
        if let Some(limit) = self.regex_size_limit {
            builder.size_limit(limit);
        }
        if let Some(limit) = self.dfa_size_limit {
            builder.dfa_size_limit(limit);
        }
        if !self.binary.is_none() {
            builder.ban_byte(Some(b'\x00'));
        }
        let m = match builder.build_many(&self.patterns.patterns) {
            Ok(m) => m,
            Err(err) => {
                anyhow::bail!(suggest_text(suggest_multiline(err.to_string())))
            }
        };
        Ok(PatternMatcher::RustRegex(m))
    }

    /// Returns true if some non-zero number of matches is believed to be
    /// possible.
    ///
    /// When this returns false, it is impossible for ripgrep to ever report
    /// a match.
    pub(crate) fn matches_possible(&self) -> bool {
        if self.patterns.patterns.is_empty() {
            return false;
        }
        if self.max_count == Some(0) {
            return false;
        }
        true
    }

    /// Returns the "mode" that ripgrep should operate in.
    ///
    /// This is generally useful for determining what action ripgrep should
    /// take. The main mode is of course to "search," but there are other
    /// non-search modes such as `--type-list` and `--files`.
    pub(crate) fn mode(&self) -> Mode {
        self.mode
    }

    /// Returns a builder for constructing a "path printer."
    ///
    /// This is useful for the `--files` mode in ripgrep, where the printer
    /// just needs to emit paths and not need to worry about the functionality
    /// of searching.
    pub(crate) fn path_printer_builder(
        &self,
    ) -> grep::printer::PathPrinterBuilder {
        let mut builder = grep::printer::PathPrinterBuilder::new();
        builder
            .color_specs(self.colors.clone())
            .hyperlink(self.hyperlink_config.clone())
            .separator(self.path_separator.clone())
            .terminator(self.path_terminator.unwrap_or(b'\n'));
        builder
    }

    /// Returns a printer for the given search mode.
    ///
    /// This chooses which printer to build (JSON, summary or standard) based
    /// on the search mode given.
    pub(crate) fn printer<W: termcolor::WriteColor>(
        &self,
        search_mode: SearchMode,
        wtr: W,
    ) -> Printer<W> {
        let summary_kind = if self.quiet {
            SummaryKind::Quiet
        } else {
            match search_mode {
                SearchMode::FilesWithMatches => SummaryKind::PathWithMatch,
                SearchMode::FilesWithoutMatch => SummaryKind::PathWithoutMatch,
                SearchMode::Count => SummaryKind::Count,
                SearchMode::CountMatches => SummaryKind::CountMatches,
                SearchMode::JSON => {
                    return Printer::JSON(self.printer_json(wtr))
                }
                SearchMode::Standard => {
                    return Printer::Standard(self.printer_standard(wtr))
                }
            }
        };
        Printer::Summary(self.printer_summary(wtr, summary_kind))
    }

    /// Builds a JSON printer.
    fn printer_json<W: std::io::Write>(
        &self,
        wtr: W,
    ) -> grep::printer::JSON<W> {
        grep::printer::JSONBuilder::new()
            .pretty(false)
            .max_matches(self.max_count)
            .always_begin_end(false)
            .build(wtr)
    }

    /// Builds a "standard" grep printer where matches are printed as plain
    /// text lines.
    fn printer_standard<W: termcolor::WriteColor>(
        &self,
        wtr: W,
    ) -> grep::printer::Standard<W> {
        let mut builder = grep::printer::StandardBuilder::new();
        builder
            .byte_offset(self.byte_offset)
            .color_specs(self.colors.clone())
            .column(self.column)
            .heading(self.heading)
            .hyperlink(self.hyperlink_config.clone())
            .max_columns_preview(self.max_columns_preview)
            .max_columns(self.max_columns)
            .max_matches(self.max_count)
            .only_matching(self.only_matching)
            .path(self.with_filename)
            .path_terminator(self.path_terminator.clone())
            .per_match_one_line(true)
            .per_match(self.vimgrep)
            .replacement(self.replace.clone().map(|r| r.into()))
            .separator_context(self.context_separator.clone().into_bytes())
            .separator_field_context(
                self.field_context_separator.clone().into_bytes(),
            )
            .separator_field_match(
                self.field_match_separator.clone().into_bytes(),
            )
            .separator_path(self.path_separator.clone())
            .stats(self.stats.is_some())
            .trim_ascii(self.trim);
        // When doing multi-threaded searching, the buffer writer is
        // responsible for writing separators since it is the only thing that
        // knows whether something has been printed or not. But for the single
        // threaded case, we don't use a buffer writer and thus can let the
        // printer own this.
        if self.threads == 1 {
            builder.separator_search(self.file_separator.clone());
        }
        builder.build(wtr)
    }

    /// Builds a "summary" printer where search results are aggregated on a
    /// file-by-file basis.
    fn printer_summary<W: termcolor::WriteColor>(
        &self,
        wtr: W,
        kind: SummaryKind,
    ) -> grep::printer::Summary<W> {
        grep::printer::SummaryBuilder::new()
            .color_specs(self.colors.clone())
            .exclude_zero(!self.include_zero)
            .hyperlink(self.hyperlink_config.clone())
            .kind(kind)
            .max_matches(self.max_count)
            .path(self.with_filename)
            .path_terminator(self.path_terminator.clone())
            .separator_field(b":".to_vec())
            .separator_path(self.path_separator.clone())
            .stats(self.stats.is_some())
            .build(wtr)
    }

    /// Returns true if ripgrep should operate in "quiet" mode.
    ///
    /// Generally speaking, quiet mode means that ripgrep should not print
    /// anything to stdout. There are some exceptions. For example, when the
    /// user has provided `--stats`, then ripgrep will print statistics to
    /// stdout.
    pub(crate) fn quiet(&self) -> bool {
        self.quiet
    }

    /// Returns true when ripgrep should stop searching after a single match is
    /// found.
    ///
    /// This is useful for example when quiet mode is enabled. In that case,
    /// users generally can't tell the difference in behavior between a search
    /// that finds all matches and a search that only finds one of them. (An
    /// exception here is if `--stats` is given, then `quit_after_match` will
    /// always return false since the user expects ripgrep to find everything.)
    pub(crate) fn quit_after_match(&self) -> bool {
        self.quit_after_match
    }

    /// Build a worker for executing searches.
    ///
    /// Search results are found using the given matcher and written to the
    /// given printer.
    pub(crate) fn search_worker<W: termcolor::WriteColor>(
        &self,
        matcher: PatternMatcher,
        searcher: grep::searcher::Searcher,
        printer: Printer<W>,
    ) -> anyhow::Result<SearchWorker<W>> {
        let mut builder = SearchWorkerBuilder::new();
        builder
            .preprocessor(self.pre.clone())?
            .preprocessor_globs(self.pre_globs.clone())
            .search_zip(self.search_zip)
            .binary_detection_explicit(self.binary.explicit.clone())
            .binary_detection_implicit(self.binary.implicit.clone());
        Ok(builder.build(matcher, searcher, printer))
    }

    /// Build a searcher from the command line parameters.
    pub(crate) fn searcher(&self) -> anyhow::Result<grep::searcher::Searcher> {
        let line_term = if self.crlf {
            grep::matcher::LineTerminator::crlf()
        } else if self.null_data {
            grep::matcher::LineTerminator::byte(b'\x00')
        } else {
            grep::matcher::LineTerminator::byte(b'\n')
        };
        let mut builder = grep::searcher::SearcherBuilder::new();
        builder
            .line_terminator(line_term)
            .invert_match(self.invert_match)
            .line_number(self.line_number)
            .multi_line(self.multiline)
            .memory_map(self.mmap_choice.clone())
            .stop_on_nonmatch(self.stop_on_nonmatch);
        match self.context {
            ContextMode::Passthru => {
                builder.passthru(true);
            }
            ContextMode::Limited(ref limited) => {
                let (before, after) = limited.get();
                builder.before_context(before);
                builder.after_context(after);
            }
        }
        match self.encoding {
            EncodingMode::Auto => {} // default for the searcher
            EncodingMode::Some(ref enc) => {
                builder.encoding(Some(enc.clone()));
            }
            EncodingMode::Disabled => {
                builder.bom_sniffing(false);
            }
        }
        Ok(builder.build())
    }

    /// Given an iterator of haystacks, sort them if necessary.
    ///
    /// When sorting is necessary, this will collect the entire iterator into
    /// memory, sort them and then return a new iterator. When sorting is not
    /// necessary, then the iterator given is returned as is without collecting
    /// it into memory.
    ///
    /// Once special case is when sorting by path in ascending order has been
    /// requested. In this case, the iterator given is returned as is without
    /// any additional sorting. This is done because `walk_builder()` will sort
    /// the iterator it yields during directory traversal, so no additional
    /// sorting is needed.
    pub(crate) fn sort<'a, I>(
        &self,
        haystacks: I,
    ) -> Box<dyn Iterator<Item = Haystack> + 'a>
    where
        I: Iterator<Item = Haystack> + 'a,
    {
        use std::{cmp::Ordering, fs::Metadata, io, time::SystemTime};

        fn attach_timestamps(
            haystacks: impl Iterator<Item = Haystack>,
            get: impl Fn(&Metadata) -> io::Result<SystemTime>,
        ) -> impl Iterator<Item = (Haystack, Option<SystemTime>)> {
            haystacks.map(move |s| {
                let time = s.path().metadata().and_then(|m| get(&m)).ok();
                (s, time)
            })
        }

        let Some(ref sort) = self.sort else { return Box::new(haystacks) };
        let mut with_timestamps: Vec<_> = match sort.kind {
            SortModeKind::Path if !sort.reverse => return Box::new(haystacks),
            SortModeKind::Path => {
                let mut haystacks = haystacks.collect::<Vec<Haystack>>();
                haystacks.sort_by(|ref h1, ref h2| {
                    h1.path().cmp(h2.path()).reverse()
                });
                return Box::new(haystacks.into_iter());
            }
            SortModeKind::LastModified => {
                attach_timestamps(haystacks, |md| md.modified()).collect()
            }
            SortModeKind::LastAccessed => {
                attach_timestamps(haystacks, |md| md.accessed()).collect()
            }
            SortModeKind::Created => {
                attach_timestamps(haystacks, |md| md.created()).collect()
            }
        };
        with_timestamps.sort_by(|(_, ref t1), (_, ref t2)| {
            let ordering = match (*t1, *t2) {
                // Both have metadata, do the obvious thing.
                (Some(t1), Some(t2)) => t1.cmp(&t2),
                // Things that error should appear later (when ascending).
                (Some(_), None) => Ordering::Less,
                // Things that error should appear later (when ascending).
                (None, Some(_)) => Ordering::Greater,
                // When both error, we can't distinguish, so treat as equal.
                (None, None) => Ordering::Equal,
            };
            if sort.reverse {
                ordering.reverse()
            } else {
                ordering
            }
        });
        Box::new(with_timestamps.into_iter().map(|(s, _)| s))
    }

    /// Returns a stats object if the user requested that ripgrep keep track
    /// of various metrics during a search.
    ///
    /// When this returns `None`, then callers may assume that the user did
    /// not request statistics.
    pub(crate) fn stats(&self) -> Option<grep::printer::Stats> {
        self.stats.clone()
    }

    /// Returns a color-enabled writer for stdout.
    ///
    /// The writer returned is also configured to do either line or block
    /// buffering, based on either explicit configuration from the user via CLI
    /// flags, or automatically based on whether stdout is connected to a tty.
    pub(crate) fn stdout(&self) -> grep::cli::StandardStream {
        let color = self.color.to_termcolor();
        match self.buffer {
            BufferMode::Auto => {
                if self.is_terminal_stdout {
                    grep::cli::stdout_buffered_line(color)
                } else {
                    grep::cli::stdout_buffered_block(color)
                }
            }
            BufferMode::Line => grep::cli::stdout_buffered_line(color),
            BufferMode::Block => grep::cli::stdout_buffered_block(color),
        }
    }

    /// Returns the total number of threads ripgrep should use to execute a
    /// search.
    ///
    /// This number is the result of reasoning about both heuristics (like
    /// the available number of cores) and whether ripgrep's mode supports
    /// parallelism. It is intended that this number be used to directly
    /// determine how many threads to spawn.
    pub(crate) fn threads(&self) -> usize {
        self.threads
    }

    /// Returns the file type matcher that was built.
    ///
    /// The matcher includes both the default rules and any rules added by the
    /// user for this specific invocation.
    pub(crate) fn types(&self) -> &ignore::types::Types {
        &self.types
    }

    /// Create a new builder for recursive directory traversal.
    ///
    /// The builder returned can be used to start a single threaded or multi
    /// threaded directory traversal. For multi threaded traversal, the number
    /// of threads configured is equivalent to `HiArgs::threads`.
    ///
    /// If `HiArgs::threads` is equal to `1`, then callers should generally
    /// choose to explicitly use single threaded traversal since it won't have
    /// the unnecessary overhead of synchronization.
    pub(crate) fn walk_builder(&self) -> anyhow::Result<ignore::WalkBuilder> {
        let mut builder = ignore::WalkBuilder::new(&self.paths.paths[0]);
        for path in self.paths.paths.iter().skip(1) {
            builder.add(path);
        }
        if !self.no_ignore_files {
            for path in self.ignore_file.iter() {
                if let Some(err) = builder.add_ignore(path) {
                    ignore_message!("{err}");
                }
            }
        }
        builder
            .max_depth(self.max_depth)
            .follow_links(self.follow)
            .max_filesize(self.max_filesize)
            .threads(self.threads)
            .same_file_system(self.one_file_system)
            .skip_stdout(matches!(self.mode, Mode::Search(_)))
            .overrides(self.globs.clone())
            .types(self.types.clone())
            .hidden(!self.hidden)
            .parents(!self.no_ignore_parent)
            .ignore(!self.no_ignore_dot)
            .git_global(!self.no_ignore_vcs && !self.no_ignore_global)
            .git_ignore(!self.no_ignore_vcs)
            .git_exclude(!self.no_ignore_vcs && !self.no_ignore_exclude)
            .require_git(!self.no_require_git)
            .ignore_case_insensitive(self.ignore_file_case_insensitive);
        if !self.no_ignore_dot {
            builder.add_custom_ignore_filename(".rgignore");
        }
        // When we want to sort paths lexicographically in ascending order,
        // then we can actually do this during directory traversal itself.
        // Otherwise, sorting is done by collecting all paths, sorting them and
        // then searching them.
        if let Some(ref sort) = self.sort {
            assert_eq!(1, self.threads, "sorting implies single threaded");
            if !sort.reverse && matches!(sort.kind, SortModeKind::Path) {
                builder.sort_by_file_name(|a, b| a.cmp(b));
            }
        }
        Ok(builder)
    }
}

/// State that only needs to be computed once during argument parsing.
///
/// This state is meant to be somewhat generic and shared across multiple
/// low->high argument conversions. The state can even be mutated by various
/// conversions as a way to communicate changes to other conversions. For
/// example, reading patterns might consume from stdin. If we know stdin
/// has been consumed and no other file paths have been given, then we know
/// for sure that we should search the CWD. In this way, a state change
/// when reading the patterns can impact how the file paths are ultimately
/// generated.
#[derive(Debug)]
struct State {
    /// Whether it's believed that tty is connected to stdout. Note that on
    /// unix systems, this is always correct. On Windows, heuristics are used
    /// by Rust's standard library, particularly for cygwin/MSYS environments.
    is_terminal_stdout: bool,
    /// Whether stdin has already been consumed. This is useful to know and for
    /// providing good error messages when the user has tried to read from stdin
    /// in two different places. For example, `rg -f - -`.
    stdin_consumed: bool,
    /// The current working directory.
    cwd: PathBuf,
}

impl State {
    /// Initialize state to some sensible defaults.
    ///
    /// Note that the state values may change throughout the lifetime of
    /// argument parsing.
    fn new() -> anyhow::Result<State> {
        use std::io::IsTerminal;

        Ok(State {
            is_terminal_stdout: std::io::stdout().is_terminal(),
            stdin_consumed: false,
            cwd: current_dir()?,
        })
    }
}

/// The disjunction of patterns to search for.
///
/// The number of patterns can be empty, e.g., via `-f /dev/null`.
#[derive(Debug)]
struct Patterns {
    /// The actual patterns to match.
    patterns: Vec<String>,
}

impl Patterns {
    /// Pulls the patterns out of the low arguments.
    ///
    /// This includes collecting patterns from -e/--regexp and -f/--file.
    ///
    /// If the invocation implies that the first positional argument is a
    /// pattern (the common case), then the first positional argument is
    /// extracted as well.
    fn from_low_args(
        state: &mut State,
        low: &mut LowArgs,
    ) -> anyhow::Result<Patterns> {
        // The first positional is only a pattern when ripgrep is instructed to
        // search and neither -e/--regexp nor -f/--file is given. Basically,
        // the first positional is a pattern only when a pattern hasn't been
        // given in some other way.

        // No search means no patterns. Even if -e/--regexp or -f/--file is
        // given, we know we won't use them so don't bother collecting them.
        if !matches!(low.mode, Mode::Search(_)) {
            return Ok(Patterns { patterns: vec![] });
        }
        // If we got nothing from -e/--regexp and -f/--file, then the first
        // positional is a pattern.
        if low.patterns.is_empty() {
            anyhow::ensure!(
                !low.positional.is_empty(),
                "ripgrep requires at least one pattern to execute a search"
            );
            let ospat = low.positional.remove(0);
            let Ok(pat) = ospat.into_string() else {
                anyhow::bail!("pattern given is not valid UTF-8")
            };
            return Ok(Patterns { patterns: vec![pat] });
        }
        // Otherwise, we need to slurp up our patterns from -e/--regexp and
        // -f/--file. We de-duplicate as we go. If we don't de-duplicate,
        // then it can actually lead to major slow downs for sloppy inputs.
        // This might be surprising, and the regex engine will eventually
        // de-duplicate duplicative branches in a single regex (maybe), but
        // not until after it has gone through parsing and some other layers.
        // If there are a lot of duplicates, then that can lead to a sizeable
        // extra cost. It is lamentable that we pay the extra cost here to
        // de-duplicate for a likely uncommon case, but I've seen this have a
        // big impact on real world data.
        let mut seen = HashSet::new();
        let mut patterns = Vec::with_capacity(low.patterns.len());
        let mut add = |pat: String| {
            if !seen.contains(&pat) {
                seen.insert(pat.clone());
                patterns.push(pat);
            }
        };
        for source in low.patterns.drain(..) {
            match source {
                PatternSource::Regexp(pat) => add(pat),
                PatternSource::File(path) => {
                    if path == Path::new("-") {
                        anyhow::ensure!(
                            !state.stdin_consumed,
                            "error reading -f/--file from stdin: stdin \
                             has already been consumed"
                        );
                        for pat in grep::cli::patterns_from_stdin()? {
                            add(pat);
                        }
                        state.stdin_consumed = true;
                    } else {
                        for pat in grep::cli::patterns_from_path(&path)? {
                            add(pat);
                        }
                    }
                }
            }
        }
        Ok(Patterns { patterns })
    }
}

/// The collection of paths we want to search for.
///
/// This guarantees that there is always at least one path.
#[derive(Debug)]
struct Paths {
    /// The actual paths.
    paths: Vec<PathBuf>,
    /// This is true when ripgrep had to guess to search the current working
    /// directory. e.g., When the user just runs `rg foo`. It is odd to need
    /// this, but it subtly changes how the paths are printed. When no explicit
    /// path is given, then ripgrep doesn't prefix each path with `./`. But
    /// otherwise it does! This curious behavior matches what GNU grep does.
    has_implicit_path: bool,
    /// Set to true if it is known that only a single file descriptor will
    /// be searched.
    is_one_file: bool,
}

impl Paths {
    /// Drain the search paths out of the given low arguments.
    fn from_low_args(
        state: &mut State,
        _: &Patterns,
        low: &mut LowArgs,
    ) -> anyhow::Result<Paths> {
        // We require a `&Patterns` even though we don't use it to ensure that
        // patterns have already been read from LowArgs. This let's us safely
        // assume that all remaining positional arguments are intended to be
        // file paths.

        let mut paths = Vec::with_capacity(low.positional.len());
        for osarg in low.positional.drain(..) {
            let path = PathBuf::from(osarg);
            if state.stdin_consumed && path == Path::new("-") {
                anyhow::bail!(
                    "error: attempted to read patterns from stdin \
                     while also searching stdin",
                );
            }
            paths.push(path);
        }
        log::debug!("number of paths given to search: {}", paths.len());
        if !paths.is_empty() {
            let is_one_file = paths.len() == 1
                // Note that we specifically use `!paths[0].is_dir()` here
                // instead of `paths[0].is_file()`. Namely, the latter can
                // return `false` even when the path is something resembling
                // a file. So instead, we just consider the path a file as
                // long as we know it isn't a directory.
                //
                // See: https://github.com/BurntSushi/ripgrep/issues/2736
                && (paths[0] == Path::new("-") || !paths[0].is_dir());
            log::debug!("is_one_file? {is_one_file:?}");
            return Ok(Paths { paths, has_implicit_path: false, is_one_file });
        }
        // N.B. is_readable_stdin is a heuristic! Part of the issue is that a
        // lot of "exec process" APIs will open a stdin pipe even though stdin
        // isn't really being used. ripgrep then thinks it should search stdin
        // and one gets the appearance of it hanging. It's a terrible failure
        // mode, but there really is no good way to mitigate it. It's just a
        // consequence of letting the user type 'rg foo' and "guessing" that
        // they meant to search the CWD.
        let is_readable_stdin = grep::cli::is_readable_stdin();
        let use_cwd = !is_readable_stdin
            || state.stdin_consumed
            || !matches!(low.mode, Mode::Search(_));
        log::debug!(
            "using heuristics to determine whether to read from \
             stdin or search ./ (\
             is_readable_stdin={is_readable_stdin}, \
             stdin_consumed={stdin_consumed}, \
             mode={mode:?})",
            stdin_consumed = state.stdin_consumed,
            mode = low.mode,
        );
        let (path, is_one_file) = if use_cwd {
            log::debug!("heuristic chose to search ./");
            (PathBuf::from("./"), false)
        } else {
            log::debug!("heuristic chose to search stdin");
            (PathBuf::from("-"), true)
        };
        Ok(Paths { paths: vec![path], has_implicit_path: true, is_one_file })
    }

    /// Returns true if ripgrep will only search stdin and nothing else.
    fn is_only_stdin(&self) -> bool {
        self.paths.len() == 1 && self.paths[0] == Path::new("-")
    }
}

/// The "binary detection" configuration that ripgrep should use.
///
/// ripgrep actually uses two different binary detection heuristics depending
/// on whether a file is explicitly being searched (e.g., via a CLI argument)
/// or implicitly searched (e.g., via directory traversal). In general, the
/// former can never use a heuristic that lets it "quit" seaching before
/// either getting EOF or finding a match. (Because doing otherwise would be
/// considered a filter, and ripgrep follows the rule that an explicitly given
/// file is always searched.)
#[derive(Debug)]
struct BinaryDetection {
    explicit: grep::searcher::BinaryDetection,
    implicit: grep::searcher::BinaryDetection,
}

impl BinaryDetection {
    /// Determines the correct binary detection mode from low-level arguments.
    fn from_low_args(_: &State, low: &LowArgs) -> BinaryDetection {
        let none = matches!(low.binary, BinaryMode::AsText) || low.null_data;
        let convert = matches!(low.binary, BinaryMode::SearchAndSuppress);
        let explicit = if none {
            grep::searcher::BinaryDetection::none()
        } else {
            grep::searcher::BinaryDetection::convert(b'\x00')
        };
        let implicit = if none {
            grep::searcher::BinaryDetection::none()
        } else if convert {
            grep::searcher::BinaryDetection::convert(b'\x00')
        } else {
            grep::searcher::BinaryDetection::quit(b'\x00')
        };
        BinaryDetection { explicit, implicit }
    }

    /// Returns true when both implicit and explicit binary detection is
    /// disabled.
    pub(crate) fn is_none(&self) -> bool {
        let none = grep::searcher::BinaryDetection::none();
        self.explicit == none && self.implicit == none
    }
}

/// Builds the file type matcher from low level arguments.
fn types(low: &LowArgs) -> anyhow::Result<ignore::types::Types> {
    let mut builder = ignore::types::TypesBuilder::new();
    builder.add_defaults();
    for tychange in low.type_changes.iter() {
        match tychange {
            TypeChange::Clear { ref name } => {
                builder.clear(name);
            }
            TypeChange::Add { ref def } => {
                builder.add_def(def)?;
            }
            TypeChange::Select { ref name } => {
                builder.select(name);
            }
            TypeChange::Negate { ref name } => {
                builder.negate(name);
            }
        }
    }
    Ok(builder.build()?)
}

/// Builds the glob "override" matcher from the CLI `-g/--glob` and `--iglob`
/// flags.
fn globs(
    state: &State,
    low: &LowArgs,
) -> anyhow::Result<ignore::overrides::Override> {
    if low.globs.is_empty() && low.iglobs.is_empty() {
        return Ok(ignore::overrides::Override::empty());
    }
    let mut builder = ignore::overrides::OverrideBuilder::new(&state.cwd);
    // Make all globs case insensitive with --glob-case-insensitive.
    if low.glob_case_insensitive {
        builder.case_insensitive(true).unwrap();
    }
    for glob in low.globs.iter() {
        builder.add(glob)?;
    }
    // This only enables case insensitivity for subsequent globs.
    builder.case_insensitive(true).unwrap();
    for glob in low.iglobs.iter() {
        builder.add(&glob)?;
    }
    Ok(builder.build()?)
}

/// Builds a glob matcher for all of the preprocessor globs (via `--pre-glob`).
fn preprocessor_globs(
    state: &State,
    low: &LowArgs,
) -> anyhow::Result<ignore::overrides::Override> {
    if low.pre_glob.is_empty() {
        return Ok(ignore::overrides::Override::empty());
    }
    let mut builder = ignore::overrides::OverrideBuilder::new(&state.cwd);
    for glob in low.pre_glob.iter() {
        builder.add(glob)?;
    }
    Ok(builder.build()?)
}

/// Determines whether stats should be tracked for this search. If so, a stats
/// object is returned.
fn stats(low: &LowArgs) -> Option<grep::printer::Stats> {
    if !matches!(low.mode, Mode::Search(_)) {
        return None;
    }
    if low.stats || matches!(low.mode, Mode::Search(SearchMode::JSON)) {
        return Some(grep::printer::Stats::new());
    }
    None
}

/// Pulls out any color specs provided by the user and assembles them into one
/// single configuration.
fn take_color_specs(_: &mut State, low: &mut LowArgs) -> ColorSpecs {
    let mut specs = grep::printer::default_color_specs();
    for spec in low.colors.drain(..) {
        specs.push(spec);
    }
    ColorSpecs::new(&specs)
}

/// Pulls out the necessary info from the low arguments to build a full
/// hyperlink configuration.
fn take_hyperlink_config(
    _: &mut State,
    low: &mut LowArgs,
) -> anyhow::Result<grep::printer::HyperlinkConfig> {
    let mut env = grep::printer::HyperlinkEnvironment::new();
    if let Some(hostname) = hostname(low.hostname_bin.as_deref()) {
        log::debug!("found hostname for hyperlink configuration: {hostname}");
        env.host(Some(hostname));
    }
    if let Some(wsl_prefix) = wsl_prefix() {
        log::debug!(
            "found wsl_prefix for hyperlink configuration: {wsl_prefix}"
        );
        env.wsl_prefix(Some(wsl_prefix));
    }
    let fmt = std::mem::take(&mut low.hyperlink_format);
    log::debug!("hyperlink format: {:?}", fmt.to_string());
    Ok(grep::printer::HyperlinkConfig::new(env, fmt))
}

/// Attempts to discover the current working directory.
///
/// This mostly just defers to the standard library, however, such things will
/// fail if ripgrep is in a directory that no longer exists. We attempt some
/// fallback mechanisms, such as querying the PWD environment variable, but
/// otherwise return an error.
fn current_dir() -> anyhow::Result<PathBuf> {
    let err = match std::env::current_dir() {
        Err(err) => err,
        Ok(cwd) => return Ok(cwd),
    };
    if let Some(cwd) = std::env::var_os("PWD") {
        if !cwd.is_empty() {
            return Ok(PathBuf::from(cwd));
        }
    }
    anyhow::bail!(
        "failed to get current working directory: {err}\n\
         did your CWD get deleted?",
    )
}

/// Retrieves the hostname that should be used wherever a hostname is required.
///
/// Currently, this is only used in the hyperlink format.
///
/// This works by first running the given binary program (if present and with
/// no arguments) to get the hostname after trimming leading and trailing
/// whitespace. If that fails for any reason, then it falls back to getting
/// the hostname via platform specific means (e.g., `gethostname` on Unix).
///
/// The purpose of `bin` is to make it possible for end users to override how
/// ripgrep determines the hostname.
fn hostname(bin: Option<&Path>) -> Option<String> {
    let Some(bin) = bin else { return platform_hostname() };
    let bin = match grep::cli::resolve_binary(bin) {
        Ok(bin) => bin,
        Err(err) => {
            log::debug!(
                "failed to run command '{bin:?}' to get hostname \
                 (falling back to platform hostname): {err}",
            );
            return platform_hostname();
        }
    };
    let mut cmd = std::process::Command::new(&bin);
    cmd.stdin(std::process::Stdio::null());
    let rdr = match grep::cli::CommandReader::new(&mut cmd) {
        Ok(rdr) => rdr,
        Err(err) => {
            log::debug!(
                "failed to spawn command '{bin:?}' to get \
                 hostname (falling back to platform hostname): {err}",
            );
            return platform_hostname();
        }
    };
    let out = match std::io::read_to_string(rdr) {
        Ok(out) => out,
        Err(err) => {
            log::debug!(
                "failed to read output from command '{bin:?}' to get \
                 hostname (falling back to platform hostname): {err}",
            );
            return platform_hostname();
        }
    };
    let hostname = out.trim();
    if hostname.is_empty() {
        log::debug!(
            "output from command '{bin:?}' is empty after trimming \
             leading and trailing whitespace (falling back to \
             platform hostname)",
        );
        return platform_hostname();
    }
    Some(hostname.to_string())
}

/// Attempts to get the hostname by using platform specific routines.
///
/// For example, this will do `gethostname` on Unix and `GetComputerNameExW` on
/// Windows.
fn platform_hostname() -> Option<String> {
    let hostname_os = match grep::cli::hostname() {
        Ok(x) => x,
        Err(err) => {
            log::debug!("could not get hostname: {}", err);
            return None;
        }
    };
    let Some(hostname) = hostname_os.to_str() else {
        log::debug!(
            "got hostname {:?}, but it's not valid UTF-8",
            hostname_os
        );
        return None;
    };
    Some(hostname.to_string())
}

/// Returns the value for the `{wslprefix}` variable in a hyperlink format.
///
/// A WSL prefix is a share/network like thing that is meant to permit Windows
/// applications to open files stored within a WSL drive.
///
/// If a WSL distro name is unavailable, not valid UTF-8 or this isn't running
/// in a Unix environment, then this returns None.
///
/// See: <https://learn.microsoft.com/en-us/windows/wsl/filesystems>
fn wsl_prefix() -> Option<String> {
    if !cfg!(unix) {
        return None;
    }
    let distro_os = std::env::var_os("WSL_DISTRO_NAME")?;
    let Some(distro) = distro_os.to_str() else {
        log::debug!(
            "found WSL_DISTRO_NAME={:?}, but value is not UTF-8",
            distro_os
        );
        return None;
    };
    Some(format!("wsl$/{distro}"))
}

/// Possibly suggest another regex engine based on the error message given.
///
/// This inspects an error resulting from building a Rust regex matcher, and
/// if it's believed to correspond to a syntax error that another engine could
/// handle, then add a message to suggest the use of the engine flag.
fn suggest_other_engine(msg: String) -> String {
    if let Some(pcre_msg) = suggest_pcre2(&msg) {
        return pcre_msg;
    }
    msg
}

/// Possibly suggest PCRE2 based on the error message given.
///
/// Inspect an error resulting from building a Rust regex matcher, and if it's
/// believed to correspond to a syntax error that PCRE2 could handle, then
/// add a message to suggest the use of -P/--pcre2.
fn suggest_pcre2(msg: &str) -> Option<String> {
    if !cfg!(feature = "pcre2") {
        return None;
    }
    if !msg.contains("backreferences") && !msg.contains("look-around") {
        None
    } else {
        Some(format!(
            "{msg}

Consider enabling PCRE2 with the --pcre2 flag, which can handle backreferences
and look-around.",
        ))
    }
}

/// Possibly suggest multiline mode based on the error message given.
///
/// Does a bit of a hacky inspection of the given error message, and if it
/// looks like the user tried to type a literal line terminator then it will
/// return a new error message suggesting the use of -U/--multiline.
fn suggest_multiline(msg: String) -> String {
    if msg.contains("the literal") && msg.contains("not allowed") {
        format!(
            "{msg}

Consider enabling multiline mode with the --multiline flag (or -U for short).
When multiline mode is enabled, new line characters can be matched.",
        )
    } else {
        msg
    }
}

/// Possibly suggest the `-a/--text` flag.
fn suggest_text(msg: String) -> String {
    if msg.contains("pattern contains \"\\0\"") {
        format!(
            "{msg}

Consider enabling text mode with the --text flag (or -a for short). Otherwise,
binary detection is enabled and matching a NUL byte is impossible.",
        )
    } else {
        msg
    }
}
