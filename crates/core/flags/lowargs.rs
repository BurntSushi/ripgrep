/*!
Provides the definition of low level arguments from CLI flags.
*/

use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use {
    bstr::{BString, ByteVec},
    grep::printer::{HyperlinkFormat, UserColorSpec},
};

/// A collection of "low level" arguments.
///
/// The "low level" here is meant to constrain this type to be as close to the
/// actual CLI flags and arguments as possible. Namely, other than some
/// convenience types to help validate flag values and deal with overrides
/// between flags, these low level arguments do not contain any higher level
/// abstractions.
///
/// Another self-imposed constraint is that populating low level arguments
/// should not require anything other than validating what the user has
/// provided. For example, low level arguments should not contain a
/// `HyperlinkConfig`, since in order to get a full configuration, one needs to
/// discover the hostname of the current system (which might require running a
/// binary or a syscall).
///
/// Low level arguments are populated by the parser directly via the `update`
/// method on the corresponding implementation of the `Flag` trait.
#[derive(Debug, Default)]
pub(crate) struct LowArgs {
    // Essential arguments.
    pub(crate) special: Option<SpecialMode>,
    pub(crate) mode: Mode,
    pub(crate) positional: Vec<OsString>,
    pub(crate) patterns: Vec<PatternSource>,
    // Everything else, sorted lexicographically.
    pub(crate) binary: BinaryMode,
    pub(crate) boundary: Option<BoundaryMode>,
    pub(crate) buffer: BufferMode,
    pub(crate) byte_offset: bool,
    pub(crate) case: CaseMode,
    pub(crate) color: ColorChoice,
    pub(crate) colors: Vec<UserColorSpec>,
    pub(crate) column: Option<bool>,
    pub(crate) context: ContextMode,
    pub(crate) context_separator: ContextSeparator,
    pub(crate) crlf: bool,
    pub(crate) dfa_size_limit: Option<usize>,
    pub(crate) encoding: EncodingMode,
    pub(crate) engine: EngineChoice,
    pub(crate) field_context_separator: FieldContextSeparator,
    pub(crate) field_match_separator: FieldMatchSeparator,
    pub(crate) fixed_strings: bool,
    pub(crate) follow: bool,
    pub(crate) glob_case_insensitive: bool,
    pub(crate) globs: Vec<String>,
    pub(crate) heading: Option<bool>,
    pub(crate) hidden: bool,
    pub(crate) hostname_bin: Option<PathBuf>,
    pub(crate) hyperlink_format: HyperlinkFormat,
    pub(crate) iglobs: Vec<String>,
    pub(crate) ignore_file: Vec<PathBuf>,
    pub(crate) ignore_file_case_insensitive: bool,
    pub(crate) include_zero: bool,
    pub(crate) invert_match: bool,
    pub(crate) line_number: Option<bool>,
    pub(crate) logging: Option<LoggingMode>,
    pub(crate) max_columns: Option<u64>,
    pub(crate) max_columns_preview: bool,
    pub(crate) max_count: Option<u64>,
    pub(crate) max_depth: Option<usize>,
    pub(crate) max_filesize: Option<u64>,
    pub(crate) mmap: MmapMode,
    pub(crate) multiline: bool,
    pub(crate) multiline_dotall: bool,
    pub(crate) no_config: bool,
    pub(crate) no_ignore_dot: bool,
    pub(crate) no_ignore_exclude: bool,
    pub(crate) no_ignore_files: bool,
    pub(crate) no_ignore_global: bool,
    pub(crate) no_ignore_messages: bool,
    pub(crate) no_ignore_parent: bool,
    pub(crate) no_ignore_vcs: bool,
    pub(crate) no_messages: bool,
    pub(crate) no_require_git: bool,
    pub(crate) no_unicode: bool,
    pub(crate) null: bool,
    pub(crate) null_data: bool,
    pub(crate) one_file_system: bool,
    pub(crate) only_matching: bool,
    pub(crate) path_separator: Option<u8>,
    pub(crate) pre: Option<PathBuf>,
    pub(crate) pre_glob: Vec<String>,
    pub(crate) quiet: bool,
    pub(crate) regex_size_limit: Option<usize>,
    pub(crate) replace: Option<BString>,
    pub(crate) search_zip: bool,
    pub(crate) sort: Option<SortMode>,
    pub(crate) stats: bool,
    pub(crate) stop_on_nonmatch: bool,
    pub(crate) threads: Option<usize>,
    pub(crate) trim: bool,
    pub(crate) type_changes: Vec<TypeChange>,
    pub(crate) unrestricted: usize,
    pub(crate) vimgrep: bool,
    pub(crate) with_filename: Option<bool>,
}

/// A "special" mode that supercedes everything else.
///
/// When one of these modes is present, it overrides everything else and causes
/// ripgrep to short-circuit. In particular, we avoid converting low-level
/// argument types into higher level arguments types that can fail for various
/// reasons related to the environment. (Parsing the low-level arguments can
/// fail too, but usually not in a way that can't be worked around by removing
/// the corresponding arguments from the CLI command.) This is overall a hedge
/// to ensure that version and help information are basically always available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SpecialMode {
    /// Show a condensed version of "help" output. Generally speaking, this
    /// shows each flag and an extremely terse description of that flag on
    /// a single line. This corresponds to the `-h` flag.
    HelpShort,
    /// Shows a very verbose version of the "help" output. The docs for some
    /// flags will be paragraphs long. This corresponds to the `--help` flag.
    HelpLong,
    /// Show condensed version information. e.g., `ripgrep x.y.z`.
    VersionShort,
    /// Show verbose version information. Includes "short" information as well
    /// as features included in the build.
    VersionLong,
    /// Show PCRE2's version information, or an error if this version of
    /// ripgrep wasn't compiled with PCRE2 support.
    VersionPCRE2,
}

/// The overall mode that ripgrep should operate in.
///
/// If ripgrep were designed without the legacy of grep, these would probably
/// be sub-commands? Perhaps not, since they aren't as frequently used.
///
/// The point of putting these in one enum is that they are all mutually
/// exclusive and override one another.
///
/// Note that -h/--help and -V/--version are not included in this because
/// they always overrides everything else, regardless of where it appears
/// in the command line. They are treated as "special" modes that short-circuit
/// ripgrep's usual flow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    /// ripgrep will execute a search of some kind.
    Search(SearchMode),
    /// Show the files that *would* be searched, but don't actually search
    /// them.
    Files,
    /// List all file type definitions configured, including the default file
    /// types and any additional file types added to the command line.
    Types,
    /// Generate various things like the man page and completion files.
    Generate(GenerateMode),
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Search(SearchMode::Standard)
    }
}

impl Mode {
    /// Update this mode to the new mode while implementing various override
    /// semantics. For example, a search mode cannot override a non-search
    /// mode.
    pub(crate) fn update(&mut self, new: Mode) {
        match *self {
            // If we're in a search mode, then anything can override it.
            Mode::Search(_) => *self = new,
            _ => {
                // Once we're in a non-search mode, other non-search modes
                // can override it. But search modes cannot. So for example,
                // `--files -l` will still be Mode::Files.
                if !matches!(*self, Mode::Search(_)) {
                    *self = new;
                }
            }
        }
    }
}

/// The kind of search that ripgrep is going to perform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchMode {
    /// The default standard mode of operation. ripgrep looks for matches and
    /// prints them when found.
    ///
    /// There is no specific flag for this mode since it's the default. But
    /// some of the modes below, like JSON, have negation flags like --no-json
    /// that let you revert back to this default mode.
    Standard,
    /// Show files containing at least one match.
    FilesWithMatches,
    /// Show files that don't contain any matches.
    FilesWithoutMatch,
    /// Show files containing at least one match and the number of matching
    /// lines.
    Count,
    /// Show files containing at least one match and the total number of
    /// matches.
    CountMatches,
    /// Print matches in a JSON lines format.
    JSON,
}

/// The thing to generate via the --generate flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GenerateMode {
    /// Generate the raw roff used for the man page.
    Man,
    /// Completions for bash.
    CompleteBash,
    /// Completions for zsh.
    CompleteZsh,
    /// Completions for fish.
    CompleteFish,
    /// Completions for PowerShell.
    CompletePowerShell,
}

/// Indicates how ripgrep should treat binary data.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum BinaryMode {
    /// Automatically determine the binary mode to use. Essentially, when
    /// a file is searched explicitly, then it will be searched using the
    /// `SearchAndSuppress` strategy. Otherwise, it will be searched in a way
    /// that attempts to skip binary files as much as possible. That is, once
    /// a file is classified as binary, searching will immediately stop.
    #[default]
    Auto,
    /// Search files even when they have binary data, but if a match is found,
    /// suppress it and emit a warning.
    ///
    /// In this mode, `NUL` bytes are replaced with line terminators. This is
    /// a heuristic meant to reduce heap memory usage, since true binary data
    /// isn't line oriented. If one attempts to treat such data as line
    /// oriented, then one may wind up with impractically large lines. For
    /// example, many binary files contain very long runs of NUL bytes.
    SearchAndSuppress,
    /// Treat all files as if they were plain text. There's no skipping and no
    /// replacement of `NUL` bytes with line terminators.
    AsText,
}

/// Indicates what kind of boundary mode to use (line or word).
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum BoundaryMode {
    /// Only allow matches when surrounded by line bounaries.
    Line,
    /// Only allow matches when surrounded by word bounaries.
    Word,
}

/// Indicates the buffer mode that ripgrep should use when printing output.
///
/// The default is `Auto`.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum BufferMode {
    /// Select the buffer mode, 'line' or 'block', automatically based on
    /// whether stdout is connected to a tty.
    #[default]
    Auto,
    /// Flush the output buffer whenever a line terminator is seen.
    ///
    /// This is useful when wants to see search results more immediately,
    /// for example, with `tail -f`.
    Line,
    /// Flush the output buffer whenever it reaches some fixed size. The size
    /// is usually big enough to hold many lines.
    ///
    /// This is useful for maximum performance, particularly when printing
    /// lots of results.
    Block,
}

/// Indicates the case mode for how to interpret all patterns given to ripgrep.
///
/// The default is `Sensitive`.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum CaseMode {
    /// Patterns are matched case sensitively. i.e., `a` does not match `A`.
    #[default]
    Sensitive,
    /// Patterns are matched case insensitively. i.e., `a` does match `A`.
    Insensitive,
    /// Patterns are automatically matched case insensitively only when they
    /// consist of all lowercase literal characters. For example, the pattern
    /// `a` will match `A` but `A` will not match `a`.
    Smart,
}

/// Indicates whether ripgrep should include color/hyperlinks in its output.
///
/// The default is `Auto`.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum ColorChoice {
    /// Color and hyperlinks will never be used.
    Never,
    /// Color and hyperlinks will be used only when stdout is connected to a
    /// tty.
    #[default]
    Auto,
    /// Color will always be used.
    Always,
    /// Color will always be used and only ANSI escapes will be used.
    ///
    /// This only makes sense in the context of legacy Windows console APIs.
    /// At time of writing, ripgrep will try to use the legacy console APIs
    /// if ANSI coloring isn't believed to be possible. This option will force
    /// ripgrep to use ANSI coloring.
    Ansi,
}

impl ColorChoice {
    /// Convert this color choice to the corresponding termcolor type.
    pub(crate) fn to_termcolor(&self) -> termcolor::ColorChoice {
        match *self {
            ColorChoice::Never => termcolor::ColorChoice::Never,
            ColorChoice::Auto => termcolor::ColorChoice::Auto,
            ColorChoice::Always => termcolor::ColorChoice::Always,
            ColorChoice::Ansi => termcolor::ColorChoice::AlwaysAnsi,
        }
    }
}

/// Indicates the line context options ripgrep should use for output.
///
/// The default is no context at all.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ContextMode {
    /// All lines will be printed. That is, the context is unbounded.
    Passthru,
    /// Only show a certain number of lines before and after each match.
    Limited(ContextModeLimited),
}

impl Default for ContextMode {
    fn default() -> ContextMode {
        ContextMode::Limited(ContextModeLimited::default())
    }
}

impl ContextMode {
    /// Set the "before" context.
    ///
    /// If this was set to "passthru" context, then it is overridden in favor
    /// of limited context with the given value for "before" and `0` for
    /// "after."
    pub(crate) fn set_before(&mut self, lines: usize) {
        match *self {
            ContextMode::Passthru => {
                *self = ContextMode::Limited(ContextModeLimited {
                    before: Some(lines),
                    after: None,
                    both: None,
                })
            }
            ContextMode::Limited(ContextModeLimited {
                ref mut before,
                ..
            }) => *before = Some(lines),
        }
    }

    /// Set the "after" context.
    ///
    /// If this was set to "passthru" context, then it is overridden in favor
    /// of limited context with the given value for "after" and `0` for
    /// "before."
    pub(crate) fn set_after(&mut self, lines: usize) {
        match *self {
            ContextMode::Passthru => {
                *self = ContextMode::Limited(ContextModeLimited {
                    before: None,
                    after: Some(lines),
                    both: None,
                })
            }
            ContextMode::Limited(ContextModeLimited {
                ref mut after, ..
            }) => *after = Some(lines),
        }
    }

    /// Set the "both" context.
    ///
    /// If this was set to "passthru" context, then it is overridden in favor
    /// of limited context with the given value for "both" and `None` for
    /// "before" and "after".
    pub(crate) fn set_both(&mut self, lines: usize) {
        match *self {
            ContextMode::Passthru => {
                *self = ContextMode::Limited(ContextModeLimited {
                    before: None,
                    after: None,
                    both: Some(lines),
                })
            }
            ContextMode::Limited(ContextModeLimited {
                ref mut both, ..
            }) => *both = Some(lines),
        }
    }

    /// A convenience function for use in tests that returns the limited
    /// context. If this mode isn't limited, then it panics.
    #[cfg(test)]
    pub(crate) fn get_limited(&self) -> (usize, usize) {
        match *self {
            ContextMode::Passthru => unreachable!("context mode is passthru"),
            ContextMode::Limited(ref limited) => limited.get(),
        }
    }
}

/// A context mode for a finite number of lines.
///
/// Namely, this indicates that a specific number of lines (possibly zero)
/// should be shown before and/or after each matching line.
///
/// Note that there is a subtle difference between `Some(0)` and `None`. In the
/// former case, it happens when `0` is given explicitly, where as `None` is
/// the default value and occurs when no value is specified.
///
/// `both` is only set by the -C/--context flag. The reason why we don't just
/// set before = after = --context is because the before and after context
/// settings always take precedent over the -C/--context setting, regardless of
/// order. Thus, we need to keep track of them separately.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ContextModeLimited {
    before: Option<usize>,
    after: Option<usize>,
    both: Option<usize>,
}

impl ContextModeLimited {
    /// Returns the specific number of contextual lines that should be shown
    /// around each match. This takes proper precedent into account, i.e.,
    /// that `before` and `after` both partially override `both` in all cases.
    ///
    /// By default, this returns `(0, 0)`.
    pub(crate) fn get(&self) -> (usize, usize) {
        let (mut before, mut after) =
            self.both.map(|lines| (lines, lines)).unwrap_or((0, 0));
        // --before and --after always override --context, regardless
        // of where they appear relative to each other.
        if let Some(lines) = self.before {
            before = lines;
        }
        if let Some(lines) = self.after {
            after = lines;
        }
        (before, after)
    }
}

/// Represents the separator to use between non-contiguous sections of
/// contextual lines.
///
/// The default is `--`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContextSeparator(Option<BString>);

impl Default for ContextSeparator {
    fn default() -> ContextSeparator {
        ContextSeparator(Some(BString::from("--")))
    }
}

impl ContextSeparator {
    /// Create a new context separator from the user provided argument. This
    /// handles unescaping.
    pub(crate) fn new(os: &OsStr) -> anyhow::Result<ContextSeparator> {
        let Some(string) = os.to_str() else {
            anyhow::bail!(
                "separator must be valid UTF-8 (use escape sequences \
                 to provide a separator that is not valid UTF-8)"
            )
        };
        Ok(ContextSeparator(Some(Vec::unescape_bytes(string).into())))
    }

    /// Creates a new separator that intructs the printer to disable contextual
    /// separators entirely.
    pub(crate) fn disabled() -> ContextSeparator {
        ContextSeparator(None)
    }

    /// Return the raw bytes of this separator.
    ///
    /// If context separators were disabled, then this returns `None`.
    ///
    /// Note that this may return a `Some` variant with zero bytes.
    pub(crate) fn into_bytes(self) -> Option<Vec<u8>> {
        self.0.map(|sep| sep.into())
    }
}

/// The encoding mode the searcher will use.
///
/// The default is `Auto`.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum EncodingMode {
    /// Use only BOM sniffing to auto-detect an encoding.
    #[default]
    Auto,
    /// Use an explicit encoding forcefully, but let BOM sniffing override it.
    Some(grep::searcher::Encoding),
    /// Use no explicit encoding and disable all BOM sniffing. This will
    /// always result in searching the raw bytes, regardless of their
    /// true encoding.
    Disabled,
}

/// The regex engine to use.
///
/// The default is `Default`.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum EngineChoice {
    /// Uses the default regex engine: Rust's `regex` crate.
    ///
    /// (Well, technically it uses `regex-automata`, but `regex-automata` is
    /// the implementation of the `regex` crate.)
    #[default]
    Default,
    /// Dynamically select the right engine to use.
    ///
    /// This works by trying to use the default engine, and if the pattern does
    /// not compile, it switches over to the PCRE2 engine if it's available.
    Auto,
    /// Uses the PCRE2 regex engine if it's available.
    PCRE2,
}

/// The field context separator to use to between metadata for each contextual
/// line.
///
/// The default is `-`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldContextSeparator(BString);

impl Default for FieldContextSeparator {
    fn default() -> FieldContextSeparator {
        FieldContextSeparator(BString::from("-"))
    }
}

impl FieldContextSeparator {
    /// Create a new separator from the given argument value provided by the
    /// user. Unescaping it automatically handled.
    pub(crate) fn new(os: &OsStr) -> anyhow::Result<FieldContextSeparator> {
        let Some(string) = os.to_str() else {
            anyhow::bail!(
                "separator must be valid UTF-8 (use escape sequences \
                 to provide a separator that is not valid UTF-8)"
            )
        };
        Ok(FieldContextSeparator(Vec::unescape_bytes(string).into()))
    }

    /// Return the raw bytes of this separator.
    ///
    /// Note that this may return an empty `Vec`.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.0.into()
    }
}

/// The field match separator to use to between metadata for each matching
/// line.
///
/// The default is `:`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FieldMatchSeparator(BString);

impl Default for FieldMatchSeparator {
    fn default() -> FieldMatchSeparator {
        FieldMatchSeparator(BString::from(":"))
    }
}

impl FieldMatchSeparator {
    /// Create a new separator from the given argument value provided by the
    /// user. Unescaping it automatically handled.
    pub(crate) fn new(os: &OsStr) -> anyhow::Result<FieldMatchSeparator> {
        let Some(string) = os.to_str() else {
            anyhow::bail!(
                "separator must be valid UTF-8 (use escape sequences \
                 to provide a separator that is not valid UTF-8)"
            )
        };
        Ok(FieldMatchSeparator(Vec::unescape_bytes(string).into()))
    }

    /// Return the raw bytes of this separator.
    ///
    /// Note that this may return an empty `Vec`.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.0.into()
    }
}

/// The type of logging to do. `Debug` emits some details while `Trace` emits
/// much more.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum LoggingMode {
    Debug,
    Trace,
}

/// Indicates when to use memory maps.
///
/// The default is `Auto`.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) enum MmapMode {
    /// This instructs ripgrep to use heuristics for selecting when to and not
    /// to use memory maps for searching.
    #[default]
    Auto,
    /// This instructs ripgrep to always try memory maps when possible. (Memory
    /// maps are not possible to use in all circumstances, for example, for
    /// virtual files.)
    AlwaysTryMmap,
    /// Never use memory maps under any circumstances. This includes even
    /// when multi-line search is enabled where ripgrep will read the entire
    /// contents of a file on to the heap before searching it.
    Never,
}

/// Represents a source of patterns that ripgrep should search for.
///
/// The reason to unify these is so that we can retain the order of `-f/--flag`
/// and `-e/--regexp` flags relative to one another.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PatternSource {
    /// Comes from the `-e/--regexp` flag.
    Regexp(String),
    /// Comes from the `-f/--file` flag.
    File(PathBuf),
}

/// The sort criteria, if present.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SortMode {
    /// Whether to reverse the sort criteria (i.e., descending order).
    pub(crate) reverse: bool,
    /// The actual sorting criteria.
    pub(crate) kind: SortModeKind,
}

/// The criteria to use for sorting.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SortModeKind {
    /// Sort by path.
    Path,
    /// Sort by last modified time.
    LastModified,
    /// Sort by last accessed time.
    LastAccessed,
    /// Sort by creation time.
    Created,
}

impl SortMode {
    /// Checks whether the selected sort mode is supported. If it isn't, an
    /// error (hopefully explaining why) is returned.
    pub(crate) fn supported(&self) -> anyhow::Result<()> {
        match self.kind {
            SortModeKind::Path => Ok(()),
            SortModeKind::LastModified => {
                let md = std::env::current_exe()
                    .and_then(|p| p.metadata())
                    .and_then(|md| md.modified());
                let Err(err) = md else { return Ok(()) };
                anyhow::bail!(
                    "sorting by last modified isn't supported: {err}"
                );
            }
            SortModeKind::LastAccessed => {
                let md = std::env::current_exe()
                    .and_then(|p| p.metadata())
                    .and_then(|md| md.accessed());
                let Err(err) = md else { return Ok(()) };
                anyhow::bail!(
                    "sorting by last accessed isn't supported: {err}"
                );
            }
            SortModeKind::Created => {
                let md = std::env::current_exe()
                    .and_then(|p| p.metadata())
                    .and_then(|md| md.created());
                let Err(err) = md else { return Ok(()) };
                anyhow::bail!(
                    "sorting by creation time isn't supported: {err}"
                );
            }
        }
    }
}

/// A single instance of either a change or a selection of one ripgrep's
/// file types.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum TypeChange {
    /// Clear the given type from ripgrep.
    Clear { name: String },
    /// Add the given type definition (name and glob) to ripgrep.
    Add { def: String },
    /// Select the given type for filtering.
    Select { name: String },
    /// Select the given type for filtering but negate it.
    Negate { name: String },
}
