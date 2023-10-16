/*!
Defines a very high level "search worker" abstraction.

A search worker manages the high level interaction points between the matcher
(i.e., which regex engine is used), the searcher (i.e., how data is actually
read and matched using the regex engine) and the printer. For example, the
search worker is where things like preprocessors or decompression happens.
*/

use std::{io, path::Path};

use {grep::matcher::Matcher, termcolor::WriteColor};

/// The configuration for the search worker.
///
/// Among a few other things, the configuration primarily controls the way we
/// show search results to users at a very high level.
#[derive(Clone, Debug)]
struct Config {
    preprocessor: Option<std::path::PathBuf>,
    preprocessor_globs: ignore::overrides::Override,
    search_zip: bool,
    binary_implicit: grep::searcher::BinaryDetection,
    binary_explicit: grep::searcher::BinaryDetection,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            preprocessor: None,
            preprocessor_globs: ignore::overrides::Override::empty(),
            search_zip: false,
            binary_implicit: grep::searcher::BinaryDetection::none(),
            binary_explicit: grep::searcher::BinaryDetection::none(),
        }
    }
}

/// A builder for configuring and constructing a search worker.
#[derive(Clone, Debug)]
pub(crate) struct SearchWorkerBuilder {
    config: Config,
    command_builder: grep::cli::CommandReaderBuilder,
    decomp_builder: grep::cli::DecompressionReaderBuilder,
}

impl Default for SearchWorkerBuilder {
    fn default() -> SearchWorkerBuilder {
        SearchWorkerBuilder::new()
    }
}

impl SearchWorkerBuilder {
    /// Create a new builder for configuring and constructing a search worker.
    pub(crate) fn new() -> SearchWorkerBuilder {
        let mut cmd_builder = grep::cli::CommandReaderBuilder::new();
        cmd_builder.async_stderr(true);

        let mut decomp_builder = grep::cli::DecompressionReaderBuilder::new();
        decomp_builder.async_stderr(true);

        SearchWorkerBuilder {
            config: Config::default(),
            command_builder: cmd_builder,
            decomp_builder,
        }
    }

    /// Create a new search worker using the given searcher, matcher and
    /// printer.
    pub(crate) fn build<W: WriteColor>(
        &self,
        matcher: PatternMatcher,
        searcher: grep::searcher::Searcher,
        printer: Printer<W>,
    ) -> SearchWorker<W> {
        let config = self.config.clone();
        let command_builder = self.command_builder.clone();
        let decomp_builder = self.decomp_builder.clone();
        SearchWorker {
            config,
            command_builder,
            decomp_builder,
            matcher,
            searcher,
            printer,
        }
    }

    /// Set the path to a preprocessor command.
    ///
    /// When this is set, instead of searching files directly, the given
    /// command will be run with the file path as the first argument, and the
    /// output of that command will be searched instead.
    pub(crate) fn preprocessor(
        &mut self,
        cmd: Option<std::path::PathBuf>,
    ) -> anyhow::Result<&mut SearchWorkerBuilder> {
        if let Some(ref prog) = cmd {
            let bin = grep::cli::resolve_binary(prog)?;
            self.config.preprocessor = Some(bin);
        } else {
            self.config.preprocessor = None;
        }
        Ok(self)
    }

    /// Set the globs for determining which files should be run through the
    /// preprocessor. By default, with no globs and a preprocessor specified,
    /// every file is run through the preprocessor.
    pub(crate) fn preprocessor_globs(
        &mut self,
        globs: ignore::overrides::Override,
    ) -> &mut SearchWorkerBuilder {
        self.config.preprocessor_globs = globs;
        self
    }

    /// Enable the decompression and searching of common compressed files.
    ///
    /// When enabled, if a particular file path is recognized as a compressed
    /// file, then it is decompressed before searching.
    ///
    /// Note that if a preprocessor command is set, then it overrides this
    /// setting.
    pub(crate) fn search_zip(
        &mut self,
        yes: bool,
    ) -> &mut SearchWorkerBuilder {
        self.config.search_zip = yes;
        self
    }

    /// Set the binary detection that should be used when searching files
    /// found via a recursive directory search.
    ///
    /// Generally, this binary detection may be
    /// `grep::searcher::BinaryDetection::quit` if we want to skip binary files
    /// completely.
    ///
    /// By default, no binary detection is performed.
    pub(crate) fn binary_detection_implicit(
        &mut self,
        detection: grep::searcher::BinaryDetection,
    ) -> &mut SearchWorkerBuilder {
        self.config.binary_implicit = detection;
        self
    }

    /// Set the binary detection that should be used when searching files
    /// explicitly supplied by an end user.
    ///
    /// Generally, this binary detection should NOT be
    /// `grep::searcher::BinaryDetection::quit`, since we never want to
    /// automatically filter files supplied by the end user.
    ///
    /// By default, no binary detection is performed.
    pub(crate) fn binary_detection_explicit(
        &mut self,
        detection: grep::searcher::BinaryDetection,
    ) -> &mut SearchWorkerBuilder {
        self.config.binary_explicit = detection;
        self
    }
}

/// The result of executing a search.
///
/// Generally speaking, the "result" of a search is sent to a printer, which
/// writes results to an underlying writer such as stdout or a file. However,
/// every search also has some aggregate statistics or meta data that may be
/// useful to higher level routines.
#[derive(Clone, Debug, Default)]
pub(crate) struct SearchResult {
    has_match: bool,
    stats: Option<grep::printer::Stats>,
}

impl SearchResult {
    /// Whether the search found a match or not.
    pub(crate) fn has_match(&self) -> bool {
        self.has_match
    }

    /// Return aggregate search statistics for a single search, if available.
    ///
    /// It can be expensive to compute statistics, so these are only present
    /// if explicitly enabled in the printer provided by the caller.
    pub(crate) fn stats(&self) -> Option<&grep::printer::Stats> {
        self.stats.as_ref()
    }
}

/// The pattern matcher used by a search worker.
#[derive(Clone, Debug)]
pub(crate) enum PatternMatcher {
    RustRegex(grep::regex::RegexMatcher),
    #[cfg(feature = "pcre2")]
    PCRE2(grep::pcre2::RegexMatcher),
}

/// The printer used by a search worker.
///
/// The `W` type parameter refers to the type of the underlying writer.
#[derive(Clone, Debug)]
pub(crate) enum Printer<W> {
    /// Use the standard printer, which supports the classic grep-like format.
    Standard(grep::printer::Standard<W>),
    /// Use the summary printer, which supports aggregate displays of search
    /// results.
    Summary(grep::printer::Summary<W>),
    /// A JSON printer, which emits results in the JSON Lines format.
    JSON(grep::printer::JSON<W>),
}

impl<W: WriteColor> Printer<W> {
    /// Return a mutable reference to the underlying printer's writer.
    pub(crate) fn get_mut(&mut self) -> &mut W {
        match *self {
            Printer::Standard(ref mut p) => p.get_mut(),
            Printer::Summary(ref mut p) => p.get_mut(),
            Printer::JSON(ref mut p) => p.get_mut(),
        }
    }
}

/// A worker for executing searches.
///
/// It is intended for a single worker to execute many searches, and is
/// generally intended to be used from a single thread. When searching using
/// multiple threads, it is better to create a new worker for each thread.
#[derive(Clone, Debug)]
pub(crate) struct SearchWorker<W> {
    config: Config,
    command_builder: grep::cli::CommandReaderBuilder,
    decomp_builder: grep::cli::DecompressionReaderBuilder,
    matcher: PatternMatcher,
    searcher: grep::searcher::Searcher,
    printer: Printer<W>,
}

impl<W: WriteColor> SearchWorker<W> {
    /// Execute a search over the given haystack.
    pub(crate) fn search(
        &mut self,
        haystack: &crate::haystack::Haystack,
    ) -> io::Result<SearchResult> {
        let bin = if haystack.is_explicit() {
            self.config.binary_explicit.clone()
        } else {
            self.config.binary_implicit.clone()
        };
        let path = haystack.path();
        log::trace!("{}: binary detection: {:?}", path.display(), bin);

        self.searcher.set_binary_detection(bin);
        if haystack.is_stdin() {
            self.search_reader(path, &mut io::stdin().lock())
        } else if self.should_preprocess(path) {
            self.search_preprocessor(path)
        } else if self.should_decompress(path) {
            self.search_decompress(path)
        } else {
            self.search_path(path)
        }
    }

    /// Return a mutable reference to the underlying printer.
    pub(crate) fn printer(&mut self) -> &mut Printer<W> {
        &mut self.printer
    }

    /// Returns true if and only if the given file path should be
    /// decompressed before searching.
    fn should_decompress(&self, path: &Path) -> bool {
        if !self.config.search_zip {
            return false;
        }
        self.decomp_builder.get_matcher().has_command(path)
    }

    /// Returns true if and only if the given file path should be run through
    /// the preprocessor.
    fn should_preprocess(&self, path: &Path) -> bool {
        if !self.config.preprocessor.is_some() {
            return false;
        }
        if self.config.preprocessor_globs.is_empty() {
            return true;
        }
        !self.config.preprocessor_globs.matched(path, false).is_ignore()
    }

    /// Search the given file path by first asking the preprocessor for the
    /// data to search instead of opening the path directly.
    fn search_preprocessor(
        &mut self,
        path: &Path,
    ) -> io::Result<SearchResult> {
        use std::{fs::File, process::Stdio};

        let bin = self.config.preprocessor.as_ref().unwrap();
        let mut cmd = std::process::Command::new(bin);
        cmd.arg(path).stdin(Stdio::from(File::open(path)?));

        let mut rdr = self.command_builder.build(&mut cmd).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "preprocessor command could not start: '{:?}': {}",
                    cmd, err,
                ),
            )
        })?;
        let result = self.search_reader(path, &mut rdr).map_err(|err| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("preprocessor command failed: '{:?}': {}", cmd, err),
            )
        });
        let close_result = rdr.close();
        let search_result = result?;
        close_result?;
        Ok(search_result)
    }

    /// Attempt to decompress the data at the given file path and search the
    /// result. If the given file path isn't recognized as a compressed file,
    /// then search it without doing any decompression.
    fn search_decompress(&mut self, path: &Path) -> io::Result<SearchResult> {
        let mut rdr = self.decomp_builder.build(path)?;
        let result = self.search_reader(path, &mut rdr);
        let close_result = rdr.close();
        let search_result = result?;
        close_result?;
        Ok(search_result)
    }

    /// Search the contents of the given file path.
    fn search_path(&mut self, path: &Path) -> io::Result<SearchResult> {
        use self::PatternMatcher::*;

        let (searcher, printer) = (&mut self.searcher, &mut self.printer);
        match self.matcher {
            RustRegex(ref m) => search_path(m, searcher, printer, path),
            #[cfg(feature = "pcre2")]
            PCRE2(ref m) => search_path(m, searcher, printer, path),
        }
    }

    /// Executes a search on the given reader, which may or may not correspond
    /// directly to the contents of the given file path. Instead, the reader
    /// may actually cause something else to be searched (for example, when
    /// a preprocessor is set or when decompression is enabled). In those
    /// cases, the file path is used for visual purposes only.
    ///
    /// Generally speaking, this method should only be used when there is no
    /// other choice. Searching via `search_path` provides more opportunities
    /// for optimizations (such as memory maps).
    fn search_reader<R: io::Read>(
        &mut self,
        path: &Path,
        rdr: &mut R,
    ) -> io::Result<SearchResult> {
        use self::PatternMatcher::*;

        let (searcher, printer) = (&mut self.searcher, &mut self.printer);
        match self.matcher {
            RustRegex(ref m) => search_reader(m, searcher, printer, path, rdr),
            #[cfg(feature = "pcre2")]
            PCRE2(ref m) => search_reader(m, searcher, printer, path, rdr),
        }
    }
}

/// Search the contents of the given file path using the given matcher,
/// searcher and printer.
fn search_path<M: Matcher, W: WriteColor>(
    matcher: M,
    searcher: &mut grep::searcher::Searcher,
    printer: &mut Printer<W>,
    path: &Path,
) -> io::Result<SearchResult> {
    match *printer {
        Printer::Standard(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_path(&matcher, path, &mut sink)?;
            Ok(SearchResult {
                has_match: sink.has_match(),
                stats: sink.stats().map(|s| s.clone()),
            })
        }
        Printer::Summary(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_path(&matcher, path, &mut sink)?;
            Ok(SearchResult {
                has_match: sink.has_match(),
                stats: sink.stats().map(|s| s.clone()),
            })
        }
        Printer::JSON(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_path(&matcher, path, &mut sink)?;
            Ok(SearchResult {
                has_match: sink.has_match(),
                stats: Some(sink.stats().clone()),
            })
        }
    }
}

/// Search the contents of the given reader using the given matcher, searcher
/// and printer.
fn search_reader<M: Matcher, R: io::Read, W: WriteColor>(
    matcher: M,
    searcher: &mut grep::searcher::Searcher,
    printer: &mut Printer<W>,
    path: &Path,
    mut rdr: R,
) -> io::Result<SearchResult> {
    match *printer {
        Printer::Standard(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_reader(&matcher, &mut rdr, &mut sink)?;
            Ok(SearchResult {
                has_match: sink.has_match(),
                stats: sink.stats().map(|s| s.clone()),
            })
        }
        Printer::Summary(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_reader(&matcher, &mut rdr, &mut sink)?;
            Ok(SearchResult {
                has_match: sink.has_match(),
                stats: sink.stats().map(|s| s.clone()),
            })
        }
        Printer::JSON(ref mut p) => {
            let mut sink = p.sink_with_path(&matcher, path);
            searcher.search_reader(&matcher, &mut rdr, &mut sink)?;
            Ok(SearchResult {
                has_match: sink.has_match(),
                stats: Some(sink.stats().clone()),
            })
        }
    }
}
