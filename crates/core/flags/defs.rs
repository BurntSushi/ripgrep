/*!
Defines all of the flags available in ripgrep.

Each flag corresponds to a unit struct with a corresponding implementation
of `Flag`. Note that each implementation of `Flag` might actually have many
possible manifestations of the same "flag." That is, each implementation of
`Flag` can have the following flags available to an end user of ripgrep:

* The long flag name.
* An optional short flag name.
* An optional negated long flag name.
* An arbitrarily long list of aliases.

The idea is that even though there are multiple flags that a user can type,
one implementation of `Flag` corresponds to a single _logical_ flag inside of
ripgrep. For example, `-E`, `--encoding` and `--no-encoding` all manipulate the
same encoding state in ripgrep.
*/

use std::path::PathBuf;

use {anyhow::Context as AnyhowContext, bstr::ByteVec};

use crate::flags::{
    lowargs::{
        BinaryMode, BoundaryMode, BufferMode, CaseMode, ColorChoice,
        ContextMode, EncodingMode, EngineChoice, GenerateMode, LoggingMode,
        LowArgs, MmapMode, Mode, PatternSource, SearchMode, SortMode,
        SortModeKind, SpecialMode, TypeChange,
    },
    Category, Flag, FlagValue,
};

#[cfg(test)]
use crate::flags::parse::parse_low_raw;

use super::CompletionType;

/// A list of all flags in ripgrep via implementations of `Flag`.
///
/// The order of these flags matter. It determines the order of the flags in
/// the generated documentation (`-h`, `--help` and the man page) within each
/// category. (This is why the deprecated flags are last.)
pub(super) const FLAGS: &[&dyn Flag] = &[
    // -e/--regexp and -f/--file should come before anything else in the
    // same category.
    &Regexp,
    &File,
    &AfterContext,
    &BeforeContext,
    &Binary,
    &BlockBuffered,
    &ByteOffset,
    &CaseSensitive,
    &Color,
    &Colors,
    &Column,
    &Context,
    &ContextSeparator,
    &Count,
    &CountMatches,
    &Crlf,
    &Debug,
    &DfaSizeLimit,
    &Encoding,
    &Engine,
    &FieldContextSeparator,
    &FieldMatchSeparator,
    &Files,
    &FilesWithMatches,
    &FilesWithoutMatch,
    &FixedStrings,
    &Follow,
    &Generate,
    &Glob,
    &GlobCaseInsensitive,
    &Heading,
    &Help,
    &Hidden,
    &HostnameBin,
    &HyperlinkFormat,
    &IGlob,
    &IgnoreCase,
    &IgnoreFile,
    &IgnoreFileCaseInsensitive,
    &IncludeZero,
    &InvertMatch,
    &JSON,
    &LineBuffered,
    &LineNumber,
    &LineNumberNo,
    &LineRegexp,
    &MaxColumns,
    &MaxColumnsPreview,
    &MaxCount,
    &MaxDepth,
    &MaxFilesize,
    &Mmap,
    &Multiline,
    &MultilineDotall,
    &NoConfig,
    &NoIgnore,
    &NoIgnoreDot,
    &NoIgnoreExclude,
    &NoIgnoreFiles,
    &NoIgnoreGlobal,
    &NoIgnoreMessages,
    &NoIgnoreParent,
    &NoIgnoreVcs,
    &NoMessages,
    &NoRequireGit,
    &NoUnicode,
    &Null,
    &NullData,
    &OneFileSystem,
    &OnlyMatching,
    &PathSeparator,
    &Passthru,
    &PCRE2,
    &PCRE2Version,
    &Pre,
    &PreGlob,
    &Pretty,
    &Quiet,
    &RegexSizeLimit,
    &Replace,
    &SearchZip,
    &SmartCase,
    &Sort,
    &Sortr,
    &Stats,
    &StopOnNonmatch,
    &Text,
    &Threads,
    &Trace,
    &Trim,
    &Type,
    &TypeNot,
    &TypeAdd,
    &TypeClear,
    &TypeList,
    &Unrestricted,
    &Version,
    &Vimgrep,
    &WithFilename,
    &WithFilenameNo,
    &WordRegexp,
    // DEPRECATED (make them show up last in their respective categories)
    &AutoHybridRegex,
    &NoPcre2Unicode,
    &SortFiles,
];

/// -A/--after-context
#[derive(Debug)]
struct AfterContext;

impl Flag for AfterContext {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'A')
    }
    fn name_long(&self) -> &'static str {
        "after-context"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Show NUM lines after each match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show \fINUM\fP lines after each match.
.sp
This overrides the \flag{passthru} flag and partially overrides the
\flag{context} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.context.set_after(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_after_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_after(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--after-context", "5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--after-context=5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A", "5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-A5", "-A10"]).unwrap();
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-A5", "-A0"]).unwrap();
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-A5", "--passthru"]).unwrap();
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-A5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--after-context", n.as_str()]).unwrap();
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--after-context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --auto-hybrid-regex
#[derive(Debug)]
struct AutoHybridRegex;

impl Flag for AutoHybridRegex {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "auto-hybrid-regex"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-auto-hybrid-regex")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        "(DEPRECATED) Use PCRE2 if appropriate."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \flag{engine} instead.
.sp
When this flag is used, ripgrep will dynamically choose between supported regex
engines depending on the features used in a pattern. When ripgrep chooses a
regex engine, it applies that choice for every regex provided to ripgrep (e.g.,
via multiple \flag{regexp} or \flag{file} flags).
.sp
As an example of how this flag might behave, ripgrep will attempt to use
its default finite automata based regex engine whenever the pattern can be
successfully compiled with that regex engine. If PCRE2 is enabled and if the
pattern given could not be compiled with the default regex engine, then PCRE2
will be automatically used for searching. If PCRE2 isn't available, then this
flag has no effect because there is only one regex engine to choose from.
.sp
In the future, ripgrep may adjust its heuristics for how it decides which
regex engine to use. In general, the heuristics will be limited to a static
analysis of the patterns, and not to any specific runtime behavior observed
while searching files.
.sp
The primary downside of using this flag is that it may not always be obvious
which regex engine ripgrep uses, and thus, the match semantics or performance
profile of ripgrep may subtly and unexpectedly change. However, in many cases,
all regex engines will agree on what constitutes a match and it can be nice
to transparently support more advanced regex features like look-around and
backreferences without explicitly needing to enable them.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let mode = if v.unwrap_switch() {
            EngineChoice::Auto
        } else {
            EngineChoice::Default
        };
        args.engine = mode;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_auto_hybrid_regex() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--no-auto-hybrid-regex"])
            .unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args =
        parse_low_raw(["--no-auto-hybrid-regex", "--auto-hybrid-regex"])
            .unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args = parse_low_raw(["--auto-hybrid-regex", "-P"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["-P", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--engine=auto", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--engine=default", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=default"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);
}

/// -B/--before-context
#[derive(Debug)]
struct BeforeContext;

impl Flag for BeforeContext {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'B')
    }
    fn name_long(&self) -> &'static str {
        "before-context"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Show NUM lines before each match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show \fINUM\fP lines before each match.
.sp
This overrides the \flag{passthru} flag and partially overrides the
\flag{context} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.context.set_before(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_before_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_before(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--before-context", "5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--before-context=5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B", "5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-B5", "-B10"]).unwrap();
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-B5", "-B0"]).unwrap();
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-B5", "--passthru"]).unwrap();
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-B5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--before-context", n.as_str()]).unwrap();
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--before-context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --binary
#[derive(Debug)]
struct Binary;

impl Flag for Binary {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "binary"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-binary")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        "Search binary files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this flag will cause ripgrep to search binary files. By default,
ripgrep attempts to automatically skip binary files in order to improve the
relevance of results and make the search faster.
.sp
Binary files are heuristically detected based on whether they contain a
\fBNUL\fP byte or not. By default (without this flag set), once a \fBNUL\fP
byte is seen, ripgrep will stop searching the file. Usually, \fBNUL\fP bytes
occur in the beginning of most binary files. If a \fBNUL\fP byte occurs after
a match, then ripgrep will not print the match, stop searching that file, and
emit a warning that some matches are being suppressed.
.sp
In contrast, when this flag is provided, ripgrep will continue searching a
file even if a \fBNUL\fP byte is found. In particular, if a \fBNUL\fP byte is
found then ripgrep will continue searching until either a match is found or
the end of the file is reached, whichever comes sooner. If a match is found,
then ripgrep will stop and print a warning saying that the search stopped
prematurely.
.sp
If you want ripgrep to search a file without any special \fBNUL\fP byte
handling at all (and potentially print binary data to stdout), then you should
use the \flag{text} flag.
.sp
The \flag{binary} flag is a flag for controlling ripgrep's automatic filtering
mechanism. As such, it does not need to be used when searching a file
explicitly or when searching stdin. That is, it is only applicable when
recursively searching a directory.
.sp
When the \flag{unrestricted} flag is provided for a third time, then this flag
is automatically enabled.
.sp
This flag overrides the \flag{text} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.binary = if v.unwrap_switch() {
            BinaryMode::SearchAndSuppress
        } else {
            BinaryMode::Auto
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_binary() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--binary"]).unwrap();
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["--binary", "--no-binary"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--no-binary", "--binary"]).unwrap();
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["--binary", "-a"]).unwrap();
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--binary"]).unwrap();
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["-a", "--no-binary"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);
}

/// --block-buffered
#[derive(Debug)]
struct BlockBuffered;

impl Flag for BlockBuffered {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "block-buffered"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-block-buffered")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Force block buffering."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will use block buffering. That is, whenever a matching
line is found, it will be written to an in-memory buffer and will not be
written to stdout until the buffer reaches a certain size. This is the default
when ripgrep's stdout is redirected to a pipeline or a file. When ripgrep's
stdout is connected to a tty, line buffering will be used by default. Forcing
block buffering can be useful when dumping a large amount of contents to a tty.
.sp
This overrides the \flag{line-buffered} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.buffer = if v.unwrap_switch() {
            BufferMode::Block
        } else {
            BufferMode::Auto
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_block_buffered() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(BufferMode::Auto, args.buffer);

    let args = parse_low_raw(["--block-buffered"]).unwrap();
    assert_eq!(BufferMode::Block, args.buffer);

    let args =
        parse_low_raw(["--block-buffered", "--no-block-buffered"]).unwrap();
    assert_eq!(BufferMode::Auto, args.buffer);

    let args = parse_low_raw(["--block-buffered", "--line-buffered"]).unwrap();
    assert_eq!(BufferMode::Line, args.buffer);
}

/// --byte-offset
#[derive(Debug)]
struct ByteOffset;

impl Flag for ByteOffset {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'b')
    }
    fn name_long(&self) -> &'static str {
        "byte-offset"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-byte-offset")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Print the byte offset for each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print the 0-based byte offset within the input file before each line of output.
If \flag{only-matching} is specified, print the offset of the matched text
itself.
.sp
If ripgrep does transcoding, then the byte offset is in terms of the result
of transcoding and not the original data. This applies similarly to other
transformations on the data, such as decompression or a \flag{pre} filter.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.byte_offset = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_byte_offset() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.byte_offset);

    let args = parse_low_raw(["--byte-offset"]).unwrap();
    assert_eq!(true, args.byte_offset);

    let args = parse_low_raw(["-b"]).unwrap();
    assert_eq!(true, args.byte_offset);

    let args = parse_low_raw(["--byte-offset", "--no-byte-offset"]).unwrap();
    assert_eq!(false, args.byte_offset);

    let args = parse_low_raw(["--no-byte-offset", "-b"]).unwrap();
    assert_eq!(true, args.byte_offset);
}

/// -s/--case-sensitive
#[derive(Debug)]
struct CaseSensitive;

impl Flag for CaseSensitive {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b's')
    }
    fn name_long(&self) -> &'static str {
        "case-sensitive"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search case sensitively (default)."
    }
    fn doc_long(&self) -> &'static str {
        r"
Execute the search case sensitively. This is the default mode.
.sp
This is a global option that applies to all patterns given to ripgrep.
Individual patterns can still be matched case insensitively by using inline
regex flags. For example, \fB(?i)abc\fP will match \fBabc\fP case insensitively
even when this flag is used.
.sp
This flag overrides the \flag{ignore-case} and \flag{smart-case} flags.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "flag has no negation");
        args.case = CaseMode::Sensitive;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_case_sensitive() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--case-sensitive"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-s"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);
}

/// --color
#[derive(Debug)]
struct Color;

impl Flag for Color {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "color"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("WHEN")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "When to use color."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag controls when to use colors. The default setting is \fBauto\fP, which
means ripgrep will try to guess when to use colors. For example, if ripgrep is
printing to a tty, then it will use colors, but if it is redirected to a file
or a pipe, then it will suppress color output.
.sp
ripgrep will suppress color output by default in some other circumstances as
well. These include, but are not limited to:
.sp
.IP \(bu 3n
When the \fBTERM\fP environment variable is not set or set to \fBdumb\fP.
.sp
.IP \(bu 3n
When the \fBNO_COLOR\fP environment variable is set (regardless of value).
.sp
.IP \(bu 3n
When flags that imply no use for colors are given. For example,
\flag{vimgrep} and \flag{json}.
.
.PP
The possible values for this flag are:
.sp
.IP \fBnever\fP 10n
Colors will never be used.
.sp
.IP \fBauto\fP 10n
The default. ripgrep tries to be smart.
.sp
.IP \fBalways\fP 10n
Colors will always be used regardless of where output is sent.
.sp
.IP \fBansi\fP 10n
Like 'always', but emits ANSI escapes (even in a Windows console).
.
.PP
This flag also controls whether hyperlinks are emitted. For example, when
a hyperlink format is specified, hyperlinks won't be used when color is
suppressed. If one wants to emit hyperlinks but no colors, then one must use
the \flag{colors} flag to manually set all color styles to \fBnone\fP:
.sp
.EX
    \-\-colors 'path:none' \\
    \-\-colors 'line:none' \\
    \-\-colors 'column:none' \\
    \-\-colors 'match:none'
.EE
.sp
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["never", "auto", "always", "ansi"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.color = match convert::str(&v.unwrap_value())? {
            "never" => ColorChoice::Never,
            "auto" => ColorChoice::Auto,
            "always" => ColorChoice::Always,
            "ansi" => ColorChoice::Ansi,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_color() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(ColorChoice::Auto, args.color);

    let args = parse_low_raw(["--color", "never"]).unwrap();
    assert_eq!(ColorChoice::Never, args.color);

    let args = parse_low_raw(["--color", "auto"]).unwrap();
    assert_eq!(ColorChoice::Auto, args.color);

    let args = parse_low_raw(["--color", "always"]).unwrap();
    assert_eq!(ColorChoice::Always, args.color);

    let args = parse_low_raw(["--color", "ansi"]).unwrap();
    assert_eq!(ColorChoice::Ansi, args.color);

    let args = parse_low_raw(["--color=never"]).unwrap();
    assert_eq!(ColorChoice::Never, args.color);

    let args =
        parse_low_raw(["--color", "always", "--color", "never"]).unwrap();
    assert_eq!(ColorChoice::Never, args.color);

    let args =
        parse_low_raw(["--color", "never", "--color", "always"]).unwrap();
    assert_eq!(ColorChoice::Always, args.color);

    let result = parse_low_raw(["--color", "foofoo"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--color", "Always"]);
    assert!(result.is_err(), "{result:?}");
}

/// --colors
#[derive(Debug)]
struct Colors;

impl Flag for Colors {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "colors"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COLOR_SPEC")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Configure color settings and styles."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag specifies color settings for use in the output. This flag may be
provided multiple times. Settings are applied iteratively. Pre-existing color
labels are limited to one of eight choices: \fBred\fP, \fBblue\fP, \fBgreen\fP,
\fBcyan\fP, \fBmagenta\fP, \fByellow\fP, \fBwhite\fP and \fBblack\fP. Styles
are limited to \fBnobold\fP, \fBbold\fP, \fBnointense\fP, \fBintense\fP,
\fBnounderline\fP or \fBunderline\fP.
.sp
The format of the flag is
\fB{\fP\fItype\fP\fB}:{\fP\fIattribute\fP\fB}:{\fP\fIvalue\fP\fB}\fP.
\fItype\fP should be one of \fBpath\fP, \fBline\fP, \fBcolumn\fP or
\fBmatch\fP. \fIattribute\fP can be \fBfg\fP, \fBbg\fP or \fBstyle\fP.
\fIvalue\fP is either a color (for \fBfg\fP and \fBbg\fP) or a text style. A
special format, \fB{\fP\fItype\fP\fB}:none\fP, will clear all color settings
for \fItype\fP.
.sp
For example, the following command will change the match color to magenta and
the background color for line numbers to yellow:
.sp
.EX
    rg \-\-colors 'match:fg:magenta' \-\-colors 'line:bg:yellow'
.EE
.sp
Extended colors can be used for \fIvalue\fP when the tty supports ANSI color
sequences. These are specified as either \fIx\fP (256-color) or
.IB x , x , x
(24-bit truecolor) where \fIx\fP is a number between \fB0\fP and \fB255\fP
inclusive. \fIx\fP may be given as a normal decimal number or a hexadecimal
number, which is prefixed by \fB0x\fP.
.sp
For example, the following command will change the match background color to
that represented by the rgb value (0,128,255):
.sp
.EX
    rg \-\-colors 'match:bg:0,128,255'
.EE
.sp
or, equivalently,
.sp
.EX
    rg \-\-colors 'match:bg:0x0,0x80,0xFF'
.EE
.sp
Note that the \fBintense\fP and \fBnointense\fP styles will have no effect when
used alongside these extended color codes.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let v = convert::str(&v)?;
        args.colors.push(v.parse()?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_colors() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert!(args.colors.is_empty());

    let args = parse_low_raw(["--colors", "match:fg:magenta"]).unwrap();
    assert_eq!(args.colors, vec!["match:fg:magenta".parse().unwrap()]);

    let args = parse_low_raw([
        "--colors",
        "match:fg:magenta",
        "--colors",
        "line:bg:yellow",
    ])
    .unwrap();
    assert_eq!(
        args.colors,
        vec![
            "match:fg:magenta".parse().unwrap(),
            "line:bg:yellow".parse().unwrap()
        ]
    );
}

/// --column
#[derive(Debug)]
struct Column;

impl Flag for Column {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "column"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-column")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Show column numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show column numbers (1-based). This only shows the column numbers for the first
match on each line. This does not try to account for Unicode. One byte is equal
to one column. This implies \flag{line-number}.
.sp
When \flag{only-matching} is used, then the column numbers written correspond
to the start of each match.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.column = Some(v.unwrap_switch());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_column() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.column);

    let args = parse_low_raw(["--column"]).unwrap();
    assert_eq!(Some(true), args.column);

    let args = parse_low_raw(["--column", "--no-column"]).unwrap();
    assert_eq!(Some(false), args.column);

    let args = parse_low_raw(["--no-column", "--column"]).unwrap();
    assert_eq!(Some(true), args.column);
}

/// -C/--context
#[derive(Debug)]
struct Context;

impl Flag for Context {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'C')
    }
    fn name_long(&self) -> &'static str {
        "context"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show NUM lines before and after each match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show \fINUM\fP lines before and after each match. This is equivalent to
providing both the \flag{before-context} and \flag{after-context} flags with
the same value.
.sp
This overrides the \flag{passthru} flag. The \flag{after-context} and
\flag{before-context} flags both partially override this flag, regardless of
the order. For example, \fB\-A2 \-C1\fP is equivalent to \fB\-A2 \-B1\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.context.set_both(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_context() {
    let mkctx = |lines| {
        let mut mode = ContextMode::default();
        mode.set_both(lines);
        mode
    };

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--context", "5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["--context=5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C", "5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let args = parse_low_raw(["-C5", "-C10"]).unwrap();
    assert_eq!(mkctx(10), args.context);

    let args = parse_low_raw(["-C5", "-C0"]).unwrap();
    assert_eq!(mkctx(0), args.context);

    let args = parse_low_raw(["-C5", "--passthru"]).unwrap();
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthru", "-C5"]).unwrap();
    assert_eq!(mkctx(5), args.context);

    let n = usize::MAX.to_string();
    let args = parse_low_raw(["--context", n.as_str()]).unwrap();
    assert_eq!(mkctx(usize::MAX), args.context);

    #[cfg(target_pointer_width = "64")]
    {
        let n = (u128::from(u64::MAX) + 1).to_string();
        let result = parse_low_raw(["--context", n.as_str()]);
        assert!(result.is_err(), "{result:?}");
    }

    // Test the interaction between -A/-B and -C. Basically, -A/-B always
    // partially overrides -C, regardless of where they appear relative to
    // each other. This behavior is also how GNU grep works, and it also makes
    // logical sense to me: -A/-B are the more specific flags.
    let args = parse_low_raw(["-A1", "-C5"]).unwrap();
    let mut mode = ContextMode::default();
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((5, 1), args.context.get_limited());

    let args = parse_low_raw(["-B1", "-C5"]).unwrap();
    let mut mode = ContextMode::default();
    mode.set_before(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((1, 5), args.context.get_limited());

    let args = parse_low_raw(["-A1", "-B2", "-C5"]).unwrap();
    let mut mode = ContextMode::default();
    mode.set_before(2);
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((2, 1), args.context.get_limited());

    // These next three are like the ones above, but with -C before -A/-B. This
    // tests that -A and -B only partially override -C. That is, -C1 -A2 is
    // equivalent to -B1 -A2.
    let args = parse_low_raw(["-C5", "-A1"]).unwrap();
    let mut mode = ContextMode::default();
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((5, 1), args.context.get_limited());

    let args = parse_low_raw(["-C5", "-B1"]).unwrap();
    let mut mode = ContextMode::default();
    mode.set_before(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((1, 5), args.context.get_limited());

    let args = parse_low_raw(["-C5", "-A1", "-B2"]).unwrap();
    let mut mode = ContextMode::default();
    mode.set_before(2);
    mode.set_after(1);
    mode.set_both(5);
    assert_eq!(mode, args.context);
    assert_eq!((2, 1), args.context.get_limited());
}

/// --context-separator
#[derive(Debug)]
struct ContextSeparator;

impl Flag for ContextSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "context-separator"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-context-separator")
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the separator for contextual chunks."
    }
    fn doc_long(&self) -> &'static str {
        r"
The string used to separate non-contiguous context lines in the output. This is
only used when one of the context flags is used (that is, \flag{after-context},
\flag{before-context} or \flag{context}). Escape sequences like \fB\\x7F\fP or
\fB\\t\fP may be used. The default value is \fB\-\-\fP.
.sp
When the context separator is set to an empty string, then a line break
is still inserted. To completely disable context separators, use the
\flag-negate{context-separator} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::flags::lowargs::ContextSeparator as Separator;

        args.context_separator = match v {
            FlagValue::Switch(true) => {
                unreachable!("flag can only be disabled")
            }
            FlagValue::Switch(false) => Separator::disabled(),
            FlagValue::Value(v) => Separator::new(&v)?,
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_context_separator() {
    use bstr::BString;

    use crate::flags::lowargs::ContextSeparator as Separator;

    let getbytes = |ctxsep: Separator| ctxsep.into_bytes().map(BString::from);

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Some(BString::from("--")), getbytes(args.context_separator));

    let args = parse_low_raw(["--context-separator", "XYZ"]).unwrap();
    assert_eq!(Some(BString::from("XYZ")), getbytes(args.context_separator));

    let args = parse_low_raw(["--no-context-separator"]).unwrap();
    assert_eq!(None, getbytes(args.context_separator));

    let args = parse_low_raw([
        "--context-separator",
        "XYZ",
        "--no-context-separator",
    ])
    .unwrap();
    assert_eq!(None, getbytes(args.context_separator));

    let args = parse_low_raw([
        "--no-context-separator",
        "--context-separator",
        "XYZ",
    ])
    .unwrap();
    assert_eq!(Some(BString::from("XYZ")), getbytes(args.context_separator));

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--context-separator", r"\xFF"]).unwrap();
    assert_eq!(Some(BString::from(b"\xFF")), getbytes(args.context_separator));

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--context-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// -c/--count
#[derive(Debug)]
struct Count;

impl Flag for Count {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'c')
    }
    fn name_long(&self) -> &'static str {
        "count"
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Show count of matching lines for each file."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag suppresses normal output and shows the number of lines that match the
given patterns for each file searched. Each file containing a match has its
path and count printed on each line. Note that unless \flag{multiline}
is enabled, this reports the number of lines that match and not the total
number of matches. In multiline mode, \flag{count} is equivalent to
\flag{count-matches}.
.sp
If only one file is given to ripgrep, then only the count is printed if there
is a match. The \flag{with-filename} flag can be used to force printing the
file path in this case. If you need a count to be printed regardless of whether
there is a match, then use \flag{include-zero}.
.sp
This overrides the \flag{count-matches} flag. Note that when \flag{count}
is combined with \flag{only-matching}, then ripgrep behaves as if
\flag{count-matches} was given.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--count can only be enabled");
        args.mode.update(Mode::Search(SearchMode::Count));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_count() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--count"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);

    let args = parse_low_raw(["-c"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);

    let args = parse_low_raw(["--count-matches", "--count"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);

    let args = parse_low_raw(["--count-matches", "-c"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::Count), args.mode);
}

/// --count-matches
#[derive(Debug)]
struct CountMatches;

impl Flag for CountMatches {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "count-matches"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        None
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Show count of every match for each file."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag suppresses normal output and shows the number of individual matches
of the given patterns for each file searched. Each file containing matches has
its path and match count printed on each line. Note that this reports the total
number of individual matches and not the number of lines that match.
.sp
If only one file is given to ripgrep, then only the count is printed if there
is a match. The \flag{with-filename} flag can be used to force printing the
file path in this case.
.sp
This overrides the \flag{count} flag. Note that when \flag{count} is combined
with \flag{only-matching}, then ripgrep behaves as if \flag{count-matches} was
given.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--count-matches can only be enabled");
        args.mode.update(Mode::Search(SearchMode::CountMatches));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_count_matches() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--count-matches"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::CountMatches), args.mode);

    let args = parse_low_raw(["--count", "--count-matches"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::CountMatches), args.mode);

    let args = parse_low_raw(["-c", "--count-matches"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::CountMatches), args.mode);
}

/// --crlf
#[derive(Debug)]
struct Crlf;

impl Flag for Crlf {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "crlf"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-crlf")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Use CRLF line terminators (nice for Windows)."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will treat CRLF (\fB\\r\\n\fP) as a line terminator
instead of just \fB\\n\fP.
.sp
Principally, this permits the line anchor assertions \fB^\fP and \fB$\fP in
regex patterns to treat CRLF, CR or LF as line terminators instead of just LF.
Note that they will never match between a CR and a LF. CRLF is treated as one
single line terminator.
.sp
When using the default regex engine, CRLF support can also be enabled inside
the pattern with the \fBR\fP flag. For example, \fB(?R:$)\fP will match just
before either CR or LF, but never between CR and LF.
.sp
This flag overrides \flag{null-data}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.crlf = v.unwrap_switch();
        if args.crlf {
            args.null_data = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_crlf() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.crlf);

    let args = parse_low_raw(["--crlf"]).unwrap();
    assert_eq!(true, args.crlf);
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--crlf", "--null-data"]).unwrap();
    assert_eq!(false, args.crlf);
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf"]).unwrap();
    assert_eq!(true, args.crlf);
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--null-data", "--no-crlf"]).unwrap();
    assert_eq!(false, args.crlf);
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf", "--no-crlf"]).unwrap();
    assert_eq!(false, args.crlf);
    assert_eq!(false, args.null_data);
}

/// --debug
#[derive(Debug)]
struct Debug;

impl Flag for Debug {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "debug"
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Show debug messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show debug messages. Please use this when filing a bug report.
.sp
The \flag{debug} flag is generally useful for figuring out why ripgrep skipped
searching a particular file. The debug messages should mention all files
skipped and why they were skipped.
.sp
To get even more debug output, use the \flag{trace} flag, which implies
\flag{debug} along with additional trace data.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--debug can only be enabled");
        args.logging = Some(LoggingMode::Debug);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_debug() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.logging);

    let args = parse_low_raw(["--debug"]).unwrap();
    assert_eq!(Some(LoggingMode::Debug), args.logging);

    let args = parse_low_raw(["--trace", "--debug"]).unwrap();
    assert_eq!(Some(LoggingMode::Debug), args.logging);
}

/// --dfa-size-limit
#[derive(Debug)]
struct DfaSizeLimit;

impl Flag for DfaSizeLimit {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "dfa-size-limit"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"The upper size limit of the regex DFA."
    }
    fn doc_long(&self) -> &'static str {
        r"
The upper size limit of the regex DFA. The default limit is something generous
for any single pattern or for many smallish patterns. This should only be
changed on very large regex inputs where the (slower) fallback regex engine may
otherwise be used if the limit is reached.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.dfa_size_limit = Some(convert::human_readable_usize(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_dfa_size_limit() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.dfa_size_limit);

    #[cfg(target_pointer_width = "64")]
    {
        let args = parse_low_raw(["--dfa-size-limit", "9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.dfa_size_limit);

        let args = parse_low_raw(["--dfa-size-limit=9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.dfa_size_limit);

        let args =
            parse_low_raw(["--dfa-size-limit=9G", "--dfa-size-limit=0"])
                .unwrap();
        assert_eq!(Some(0), args.dfa_size_limit);
    }

    let args = parse_low_raw(["--dfa-size-limit=0K"]).unwrap();
    assert_eq!(Some(0), args.dfa_size_limit);

    let args = parse_low_raw(["--dfa-size-limit=0M"]).unwrap();
    assert_eq!(Some(0), args.dfa_size_limit);

    let args = parse_low_raw(["--dfa-size-limit=0G"]).unwrap();
    assert_eq!(Some(0), args.dfa_size_limit);

    let result = parse_low_raw(["--dfa-size-limit", "9999999999999999999999"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--dfa-size-limit", "9999999999999999G"]);
    assert!(result.is_err(), "{result:?}");
}

/// -E/--encoding
#[derive(Debug)]
struct Encoding;

impl Flag for Encoding {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'E')
    }
    fn name_long(&self) -> &'static str {
        "encoding"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-encoding")
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("ENCODING")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Specify the text encoding of files to search."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specify the text encoding that ripgrep will use on all files searched. The
default value is \fBauto\fP, which will cause ripgrep to do a best effort
automatic detection of encoding on a per-file basis. Automatic detection in
this case only applies to files that begin with a UTF-8 or UTF-16 byte-order
mark (BOM). No other automatic detection is performed. One can also specify
\fBnone\fP which will then completely disable BOM sniffing and always result
in searching the raw bytes, including a BOM if it's present, regardless of its
encoding.
.sp
Other supported values can be found in the list of labels here:
\fIhttps://encoding.spec.whatwg.org/#concept-encoding-get\fP.
.sp
For more details on encoding and how ripgrep deals with it, see \fBGUIDE.md\fP.
.sp
The encoding detection that ripgrep uses can be reverted to its automatic mode
via the \flag-negate{encoding} flag.
"
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Encoding
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let value = match v {
            FlagValue::Value(v) => v,
            FlagValue::Switch(true) => {
                unreachable!("--encoding must accept a value")
            }
            FlagValue::Switch(false) => {
                args.encoding = EncodingMode::Auto;
                return Ok(());
            }
        };
        let label = convert::str(&value)?;
        args.encoding = match label {
            "auto" => EncodingMode::Auto,
            "none" => EncodingMode::Disabled,
            _ => EncodingMode::Some(grep::searcher::Encoding::new(label)?),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_encoding() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--encoding", "auto"]).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--encoding", "none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["--encoding=none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-Enone"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "none", "--no-encoding"]).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--no-encoding", "-E", "none"]).unwrap();
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "utf-16"]).unwrap();
    let enc = grep::searcher::Encoding::new("utf-16").unwrap();
    assert_eq!(EncodingMode::Some(enc), args.encoding);

    let args = parse_low_raw(["-E", "utf-16", "--no-encoding"]).unwrap();
    assert_eq!(EncodingMode::Auto, args.encoding);

    let result = parse_low_raw(["-E", "foo"]);
    assert!(result.is_err(), "{result:?}");
}

/// --engine
#[derive(Debug)]
struct Engine;

impl Flag for Engine {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "engine"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("ENGINE")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Specify which regex engine to use."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specify which regular expression engine to use. When you choose a regex engine,
it applies that choice for every regex provided to ripgrep (e.g., via multiple
\flag{regexp} or \flag{file} flags).
.sp
Accepted values are \fBdefault\fP, \fBpcre2\fP, or \fBauto\fP.
.sp
The default value is \fBdefault\fP, which is usually the fastest and should be
good for most use cases. The \fBpcre2\fP engine is generally useful when you
want to use features such as look-around or backreferences. \fBauto\fP will
dynamically choose between supported regex engines depending on the features
used in a pattern on a best effort basis.
.sp
Note that the \fBpcre2\fP engine is an optional ripgrep feature. If PCRE2
wasn't included in your build of ripgrep, then using this flag will result in
ripgrep printing an error message and exiting.
.sp
This overrides previous uses of the \flag{pcre2} and \flag{auto-hybrid-regex}
flags.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["default", "pcre2", "auto"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let string = convert::str(&v)?;
        args.engine = match string {
            "default" => EngineChoice::Default,
            "pcre2" => EngineChoice::PCRE2,
            "auto" => EngineChoice::Auto,
            _ => anyhow::bail!("unrecognized regex engine '{string}'"),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_engine() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--engine", "pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["--engine=pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args =
        parse_low_raw(["--engine=pcre2", "--auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=auto"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=default"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args =
        parse_low_raw(["--engine=pcre2", "--no-auto-hybrid-regex"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);
}

/// --field-context-separator
#[derive(Debug)]
struct FieldContextSeparator;

impl Flag for FieldContextSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "field-context-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the field context separator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the field context separator. This separator is only used when printing
contextual lines. It is used to delimit file paths, line numbers, columns and
the contextual line itself. The separator may be any number of bytes, including
zero. Escape sequences like \fB\\x7F\fP or \fB\\t\fP may be used.
.sp
The \fB-\fP character is the default value.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::flags::lowargs::FieldContextSeparator as Separator;

        args.field_context_separator = Separator::new(&v.unwrap_value())?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_field_context_separator() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(BString::from("-"), args.field_context_separator.into_bytes());

    let args = parse_low_raw(["--field-context-separator", "XYZ"]).unwrap();
    assert_eq!(
        BString::from("XYZ"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw(["--field-context-separator=XYZ"]).unwrap();
    assert_eq!(
        BString::from("XYZ"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw([
        "--field-context-separator",
        "XYZ",
        "--field-context-separator",
        "ABC",
    ])
    .unwrap();
    assert_eq!(
        BString::from("ABC"),
        args.field_context_separator.into_bytes()
    );

    let args = parse_low_raw(["--field-context-separator", r"\t"]).unwrap();
    assert_eq!(BString::from("\t"), args.field_context_separator.into_bytes());

    let args = parse_low_raw(["--field-context-separator", r"\x00"]).unwrap();
    assert_eq!(
        BString::from("\x00"),
        args.field_context_separator.into_bytes()
    );

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--field-context-separator", r"\xFF"]).unwrap();
    assert_eq!(
        BString::from(b"\xFF"),
        args.field_context_separator.into_bytes()
    );

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--field-context-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// --field-match-separator
#[derive(Debug)]
struct FieldMatchSeparator;

impl Flag for FieldMatchSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "field-match-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the field match separator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the field match separator. This separator is only used when printing
matching lines. It is used to delimit file paths, line numbers, columns and the
matching line itself. The separator may be any number of bytes, including zero.
Escape sequences like \fB\\x7F\fP or \fB\\t\fP may be used.
.sp
The \fB:\fP character is the default value.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        use crate::flags::lowargs::FieldMatchSeparator as Separator;

        args.field_match_separator = Separator::new(&v.unwrap_value())?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_field_match_separator() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(BString::from(":"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", "XYZ"]).unwrap();
    assert_eq!(BString::from("XYZ"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator=XYZ"]).unwrap();
    assert_eq!(BString::from("XYZ"), args.field_match_separator.into_bytes());

    let args = parse_low_raw([
        "--field-match-separator",
        "XYZ",
        "--field-match-separator",
        "ABC",
    ])
    .unwrap();
    assert_eq!(BString::from("ABC"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", r"\t"]).unwrap();
    assert_eq!(BString::from("\t"), args.field_match_separator.into_bytes());

    let args = parse_low_raw(["--field-match-separator", r"\x00"]).unwrap();
    assert_eq!(BString::from("\x00"), args.field_match_separator.into_bytes());

    // This checks that invalid UTF-8 can be used. This case isn't too tricky
    // to handle, because it passes the invalid UTF-8 as an escape sequence
    // that is itself valid UTF-8. It doesn't become invalid UTF-8 until after
    // the argument is parsed and then unescaped.
    let args = parse_low_raw(["--field-match-separator", r"\xFF"]).unwrap();
    assert_eq!(
        BString::from(b"\xFF"),
        args.field_match_separator.into_bytes()
    );

    // In this case, we specifically try to pass an invalid UTF-8 argument to
    // the flag. In theory we might be able to support this, but because we do
    // unescaping and because unescaping wants valid UTF-8, we do a UTF-8 check
    // on the value. Since we pass invalid UTF-8, it fails. This demonstrates
    // that the only way to use an invalid UTF-8 separator is by specifying an
    // escape sequence that is itself valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let result = parse_low_raw([
            OsStr::from_bytes(b"--field-match-separator"),
            OsStr::from_bytes(&[0xFF]),
        ]);
        assert!(result.is_err(), "{result:?}");
    }
}

/// -f/--file
#[derive(Debug)]
struct File;

impl Flag for File {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'f')
    }
    fn name_long(&self) -> &'static str {
        "file"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("PATTERNFILE")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"Search for patterns from the given file."
    }
    fn doc_long(&self) -> &'static str {
        r"
Search for patterns from the given file, with one pattern per line. When this
flag is used multiple times or in combination with the \flag{regexp} flag, then
all patterns provided are searched. Empty pattern lines will match all input
lines, and the newline is not counted as part of the pattern.
.sp
A line is printed if and only if it matches at least one of the patterns.
.sp
When \fIPATTERNFILE\fP is \fB-\fP, then \fBstdin\fP will be read for the
patterns.
.sp
When \flag{file} or \flag{regexp} is used, then ripgrep treats all positional
arguments as files or directories to search.
"
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Filename
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = PathBuf::from(v.unwrap_value());
        args.patterns.push(PatternSource::File(path));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_file() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<PatternSource>::new(), args.patterns);

    let args = parse_low_raw(["--file", "foo"]).unwrap();
    assert_eq!(vec![PatternSource::File(PathBuf::from("foo"))], args.patterns);

    let args = parse_low_raw(["--file=foo"]).unwrap();
    assert_eq!(vec![PatternSource::File(PathBuf::from("foo"))], args.patterns);

    let args = parse_low_raw(["-f", "foo"]).unwrap();
    assert_eq!(vec![PatternSource::File(PathBuf::from("foo"))], args.patterns);

    let args = parse_low_raw(["-ffoo"]).unwrap();
    assert_eq!(vec![PatternSource::File(PathBuf::from("foo"))], args.patterns);

    let args = parse_low_raw(["--file", "-foo"]).unwrap();
    assert_eq!(
        vec![PatternSource::File(PathBuf::from("-foo"))],
        args.patterns
    );

    let args = parse_low_raw(["--file=-foo"]).unwrap();
    assert_eq!(
        vec![PatternSource::File(PathBuf::from("-foo"))],
        args.patterns
    );

    let args = parse_low_raw(["-f", "-foo"]).unwrap();
    assert_eq!(
        vec![PatternSource::File(PathBuf::from("-foo"))],
        args.patterns
    );

    let args = parse_low_raw(["-f-foo"]).unwrap();
    assert_eq!(
        vec![PatternSource::File(PathBuf::from("-foo"))],
        args.patterns
    );

    let args = parse_low_raw(["--file=foo", "--file", "bar"]).unwrap();
    assert_eq!(
        vec![
            PatternSource::File(PathBuf::from("foo")),
            PatternSource::File(PathBuf::from("bar"))
        ],
        args.patterns
    );

    // We permit path arguments to be invalid UTF-8. So test that. Some of
    // these cases are tricky and depend on lexopt doing the right thing.
    //
    // We probably should add tests for this handling on Windows too, but paths
    // that are invalid UTF-16 appear incredibly rare in the Windows world.
    #[cfg(unix)]
    {
        use std::{
            ffi::{OsStr, OsString},
            os::unix::ffi::{OsStrExt, OsStringExt},
        };

        let bytes = &[b'A', 0xFF, b'Z'][..];
        let path = PathBuf::from(OsString::from_vec(bytes.to_vec()));

        let args = parse_low_raw([
            OsStr::from_bytes(b"--file"),
            OsStr::from_bytes(bytes),
        ])
        .unwrap();
        assert_eq!(vec![PatternSource::File(path.clone())], args.patterns);

        let args = parse_low_raw([
            OsStr::from_bytes(b"-f"),
            OsStr::from_bytes(bytes),
        ])
        .unwrap();
        assert_eq!(vec![PatternSource::File(path.clone())], args.patterns);

        let mut bytes = b"--file=A".to_vec();
        bytes.push(0xFF);
        bytes.push(b'Z');
        let args = parse_low_raw([OsStr::from_bytes(&bytes)]).unwrap();
        assert_eq!(vec![PatternSource::File(path.clone())], args.patterns);

        let mut bytes = b"-fA".to_vec();
        bytes.push(0xFF);
        bytes.push(b'Z');
        let args = parse_low_raw([OsStr::from_bytes(&bytes)]).unwrap();
        assert_eq!(vec![PatternSource::File(path.clone())], args.patterns);
    }
}

/// --files
#[derive(Debug)]
struct Files;

impl Flag for Files {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "files"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Print each file that would be searched."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print each file that would be searched without actually performing the search.
This is useful to determine whether a particular file is being searched or not.
.sp
This overrides \flag{type-list}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch());
        args.mode.update(Mode::Files);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_files() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--files"]).unwrap();
    assert_eq!(Mode::Files, args.mode);
}

/// -l/--files-with-matches
#[derive(Debug)]
struct FilesWithMatches;

impl Flag for FilesWithMatches {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'l')
    }
    fn name_long(&self) -> &'static str {
        "files-with-matches"
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Print the paths with at least one match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print only the paths with at least one match and suppress match contents.
.sp
This overrides \flag{files-without-match}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--files-with-matches can only be enabled");
        args.mode.update(Mode::Search(SearchMode::FilesWithMatches));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_files_with_matches() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--files-with-matches"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::FilesWithMatches), args.mode);

    let args = parse_low_raw(["-l"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::FilesWithMatches), args.mode);
}

/// -l/--files-without-match
#[derive(Debug)]
struct FilesWithoutMatch;

impl Flag for FilesWithoutMatch {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "files-without-match"
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Print the paths that contain zero matches."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print the paths that contain zero matches and suppress match contents.
.sp
This overrides \flag{files-with-matches}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(
            v.unwrap_switch(),
            "--files-without-match can only be enabled"
        );
        args.mode.update(Mode::Search(SearchMode::FilesWithoutMatch));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_files_without_match() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--files-without-match"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::FilesWithoutMatch), args.mode);

    let args =
        parse_low_raw(["--files-with-matches", "--files-without-match"])
            .unwrap();
    assert_eq!(Mode::Search(SearchMode::FilesWithoutMatch), args.mode);

    let args =
        parse_low_raw(["--files-without-match", "--files-with-matches"])
            .unwrap();
    assert_eq!(Mode::Search(SearchMode::FilesWithMatches), args.mode);
}

/// -F/--fixed-strings
#[derive(Debug)]
struct FixedStrings;

impl Flag for FixedStrings {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'F')
    }
    fn name_long(&self) -> &'static str {
        "fixed-strings"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-fixed-strings")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Treat all patterns as literals."
    }
    fn doc_long(&self) -> &'static str {
        r"
Treat all patterns as literals instead of as regular expressions. When this
flag is used, special regular expression meta characters such as \fB.(){}*+\fP
should not need be escaped.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.fixed_strings = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_fixed_strings() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.fixed_strings);

    let args = parse_low_raw(["--fixed-strings"]).unwrap();
    assert_eq!(true, args.fixed_strings);

    let args = parse_low_raw(["-F"]).unwrap();
    assert_eq!(true, args.fixed_strings);

    let args = parse_low_raw(["-F", "--no-fixed-strings"]).unwrap();
    assert_eq!(false, args.fixed_strings);

    let args = parse_low_raw(["--no-fixed-strings", "-F"]).unwrap();
    assert_eq!(true, args.fixed_strings);
}

/// -L/--follow
#[derive(Debug)]
struct Follow;

impl Flag for Follow {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'L')
    }
    fn name_long(&self) -> &'static str {
        "follow"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-follow")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Follow symbolic links."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to follow symbolic links while traversing
directories. This behavior is disabled by default. Note that ripgrep will
check for symbolic link loops and report errors if it finds one. ripgrep will
also report errors for broken links. To suppress error messages, use the
\flag{no-messages} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.follow = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_follow() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.follow);

    let args = parse_low_raw(["--follow"]).unwrap();
    assert_eq!(true, args.follow);

    let args = parse_low_raw(["-L"]).unwrap();
    assert_eq!(true, args.follow);

    let args = parse_low_raw(["-L", "--no-follow"]).unwrap();
    assert_eq!(false, args.follow);

    let args = parse_low_raw(["--no-follow", "-L"]).unwrap();
    assert_eq!(true, args.follow);
}

/// --generate
#[derive(Debug)]
struct Generate;

impl Flag for Generate {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "generate"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("KIND")
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Generate man pages and completion scripts."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to generate some special kind of output identified
by \fIKIND\fP and then quit without searching. \fIKIND\fP can be one of the
following values:
.sp
.TP 15
\fBman\fP
Generates a manual page for ripgrep in the \fBroff\fP format.
.TP 15
\fBcomplete\-bash\fP
Generates a completion script for the \fBbash\fP shell.
.TP 15
\fBcomplete\-zsh\fP
Generates a completion script for the \fBzsh\fP shell.
.TP 15
\fBcomplete\-fish\fP
Generates a completion script for the \fBfish\fP shell.
.TP 15
\fBcomplete\-powershell\fP
Generates a completion script for PowerShell.
.PP
The output is written to \fBstdout\fP. The list above may expand over time.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &[
            "man",
            "complete-bash",
            "complete-zsh",
            "complete-fish",
            "complete-powershell",
        ]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let genmode = match convert::str(&v.unwrap_value())? {
            "man" => GenerateMode::Man,
            "complete-bash" => GenerateMode::CompleteBash,
            "complete-zsh" => GenerateMode::CompleteZsh,
            "complete-fish" => GenerateMode::CompleteFish,
            "complete-powershell" => GenerateMode::CompletePowerShell,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        args.mode.update(Mode::Generate(genmode));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_generate() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--generate", "man"]).unwrap();
    assert_eq!(Mode::Generate(GenerateMode::Man), args.mode);

    let args = parse_low_raw(["--generate", "complete-bash"]).unwrap();
    assert_eq!(Mode::Generate(GenerateMode::CompleteBash), args.mode);

    let args = parse_low_raw(["--generate", "complete-zsh"]).unwrap();
    assert_eq!(Mode::Generate(GenerateMode::CompleteZsh), args.mode);

    let args = parse_low_raw(["--generate", "complete-fish"]).unwrap();
    assert_eq!(Mode::Generate(GenerateMode::CompleteFish), args.mode);

    let args = parse_low_raw(["--generate", "complete-powershell"]).unwrap();
    assert_eq!(Mode::Generate(GenerateMode::CompletePowerShell), args.mode);

    let args =
        parse_low_raw(["--generate", "complete-bash", "--generate=man"])
            .unwrap();
    assert_eq!(Mode::Generate(GenerateMode::Man), args.mode);

    let args = parse_low_raw(["--generate", "man", "-l"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::FilesWithMatches), args.mode);

    // An interesting quirk of how the modes override each other that lets
    // you get back to the "default" mode of searching.
    let args =
        parse_low_raw(["--generate", "man", "--json", "--no-json"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);
}

/// -g/--glob
#[derive(Debug)]
struct Glob;

impl Flag for Glob {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'g')
    }
    fn name_long(&self) -> &'static str {
        "glob"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("GLOB")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Include or exclude file paths."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Include or exclude files and directories for searching that match the given
glob. This always overrides any other ignore logic. Multiple glob flags may
be used. Globbing rules match \fB.gitignore\fP globs. Precede a glob with a
\fB!\fP to exclude it. If multiple globs match a file or directory, the glob
given later in the command line takes precedence.
.sp
As an extension, globs support specifying alternatives:
.BI "\-g '" ab{c,d}* '
is equivalent to
.BI "\-g " "abc " "\-g " abd.
Empty alternatives like
.BI "\-g '" ab{,c} '
are not currently supported. Note that this syntax extension is also currently
enabled in \fBgitignore\fP files, even though this syntax isn't supported by
git itself. ripgrep may disable this syntax extension in gitignore files, but
it will always remain available via the \flag{glob} flag.
.sp
When this flag is set, every file and directory is applied to it to test for
a match. For example, if you only want to search in a particular directory
\fIfoo\fP, then
.BI "\-g " foo
is incorrect because \fIfoo/bar\fP does not match
the glob \fIfoo\fP. Instead, you should use
.BI "\-g '" foo/** '.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let glob = convert::string(v.unwrap_value())?;
        args.globs.push(glob);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_glob() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<String>::new(), args.globs);

    let args = parse_low_raw(["--glob", "foo"]).unwrap();
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["--glob=foo"]).unwrap();
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["-g", "foo"]).unwrap();
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["-gfoo"]).unwrap();
    assert_eq!(vec!["foo".to_string()], args.globs);

    let args = parse_low_raw(["--glob", "-foo"]).unwrap();
    assert_eq!(vec!["-foo".to_string()], args.globs);

    let args = parse_low_raw(["--glob=-foo"]).unwrap();
    assert_eq!(vec!["-foo".to_string()], args.globs);

    let args = parse_low_raw(["-g", "-foo"]).unwrap();
    assert_eq!(vec!["-foo".to_string()], args.globs);

    let args = parse_low_raw(["-g-foo"]).unwrap();
    assert_eq!(vec!["-foo".to_string()], args.globs);
}

/// --glob-case-insensitive
#[derive(Debug)]
struct GlobCaseInsensitive;

impl Flag for GlobCaseInsensitive {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "glob-case-insensitive"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-glob-case-insensitive")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Process all glob patterns case insensitively."
    }
    fn doc_long(&self) -> &'static str {
        r"
Process all glob patterns given with the \flag{glob} flag case insensitively.
This effectively treats \flag{glob} as \flag{iglob}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.glob_case_insensitive = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_glob_case_insensitive() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.glob_case_insensitive);

    let args = parse_low_raw(["--glob-case-insensitive"]).unwrap();
    assert_eq!(true, args.glob_case_insensitive);

    let args = parse_low_raw([
        "--glob-case-insensitive",
        "--no-glob-case-insensitive",
    ])
    .unwrap();
    assert_eq!(false, args.glob_case_insensitive);

    let args = parse_low_raw([
        "--no-glob-case-insensitive",
        "--glob-case-insensitive",
    ])
    .unwrap();
    assert_eq!(true, args.glob_case_insensitive);
}

/// --heading
#[derive(Debug)]
struct Heading;

impl Flag for Heading {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "heading"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-heading")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print matches grouped by each file."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints the file path above clusters of matches from each file instead
of printing the file path as a prefix for each matched line.
.sp
This is the default mode when printing to a tty.
.sp
When \fBstdout\fP is not a tty, then ripgrep will default to the standard
grep-like format. One can force this format in Unix-like environments by
piping the output of ripgrep to \fBcat\fP. For example, \fBrg\fP \fIfoo\fP \fB|
cat\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.heading = Some(v.unwrap_switch());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_heading() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.heading);

    let args = parse_low_raw(["--heading"]).unwrap();
    assert_eq!(Some(true), args.heading);

    let args = parse_low_raw(["--no-heading"]).unwrap();
    assert_eq!(Some(false), args.heading);

    let args = parse_low_raw(["--heading", "--no-heading"]).unwrap();
    assert_eq!(Some(false), args.heading);

    let args = parse_low_raw(["--no-heading", "--heading"]).unwrap();
    assert_eq!(Some(true), args.heading);
}

/// -h/--help
#[derive(Debug)]
struct Help;

impl Flag for Help {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "help"
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'h')
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show help output."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints the help output for ripgrep.
.sp
Unlike most other flags, the behavior of the short flag, \fB\-h\fP, and the
long flag, \fB\-\-help\fP, is different. The short flag will show a condensed
help output while the long flag will show a verbose help output. The verbose
help output has complete documentation, where as the condensed help output will
show only a single line for every flag.
"
    }

    fn update(&self, v: FlagValue, _: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--help has no negation");
        // Since this flag has different semantics for -h and --help and the
        // Flag trait doesn't support encoding this sort of thing, we handle it
        // as a special case in the parser.
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_help() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.special);

    let args = parse_low_raw(["-h"]).unwrap();
    assert_eq!(Some(SpecialMode::HelpShort), args.special);

    let args = parse_low_raw(["--help"]).unwrap();
    assert_eq!(Some(SpecialMode::HelpLong), args.special);

    let args = parse_low_raw(["-h", "--help"]).unwrap();
    assert_eq!(Some(SpecialMode::HelpLong), args.special);

    let args = parse_low_raw(["--help", "-h"]).unwrap();
    assert_eq!(Some(SpecialMode::HelpShort), args.special);
}

/// -./--hidden
#[derive(Debug)]
struct Hidden;

impl Flag for Hidden {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'.')
    }
    fn name_long(&self) -> &'static str {
        "hidden"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-hidden")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Search hidden files and directories."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Search hidden files and directories. By default, hidden files and directories
are skipped. Note that if a hidden file or a directory is whitelisted in
an ignore file, then it will be searched even if this flag isn't provided.
Similarly if a hidden file or directory is given explicitly as an argument to
ripgrep.
.sp
A file or directory is considered hidden if its base name starts with a dot
character (\fB.\fP). On operating systems which support a "hidden" file
attribute, like Windows, files with this attribute are also considered hidden.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.hidden = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hidden() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.hidden);

    let args = parse_low_raw(["--hidden"]).unwrap();
    assert_eq!(true, args.hidden);

    let args = parse_low_raw(["-."]).unwrap();
    assert_eq!(true, args.hidden);

    let args = parse_low_raw(["-.", "--no-hidden"]).unwrap();
    assert_eq!(false, args.hidden);

    let args = parse_low_raw(["--no-hidden", "-."]).unwrap();
    assert_eq!(true, args.hidden);
}

/// --hostname-bin
#[derive(Debug)]
struct HostnameBin;

impl Flag for HostnameBin {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "hostname-bin"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COMMAND")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Run a program to get this system's hostname."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag controls how ripgrep determines this system's hostname. The flag's
value should correspond to an executable (either a path or something that can
be found via your system's \fBPATH\fP environment variable). When set, ripgrep
will run this executable, with no arguments, and treat its output (with leading
and trailing whitespace stripped) as your system's hostname.
.sp
When not set (the default, or the empty string), ripgrep will try to
automatically detect your system's hostname. On Unix, this corresponds
to calling \fBgethostname\fP. On Windows, this corresponds to calling
\fBGetComputerNameExW\fP to fetch the system's "physical DNS hostname."
.sp
ripgrep uses your system's hostname for producing hyperlinks.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Executable
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = PathBuf::from(v.unwrap_value());
        args.hostname_bin =
            if path.as_os_str().is_empty() { None } else { Some(path) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hostname_bin() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin", "foo"]).unwrap();
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin=foo"]).unwrap();
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);
}

/// --hyperlink-format
#[derive(Debug)]
struct HyperlinkFormat;

impl Flag for HyperlinkFormat {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "hyperlink-format"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("FORMAT")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the format of hyperlinks."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Set the format of hyperlinks to use when printing results. Hyperlinks make
certain elements of ripgrep's output, such as file paths, clickable. This
generally only works in terminal emulators that support OSC-8 hyperlinks. For
example, the format \fBfile://{host}{path}\fP will emit an RFC 8089 hyperlink.
To see the format that ripgrep is using, pass the \flag{debug} flag.
.sp
Alternatively, a format string may correspond to one of the following aliases:
\fBdefault\fP, \fBnone\fP, \fBfile\fP, \fBgrep+\fP, \fBkitty\fP, \fBmacvim\fP,
\fBtextmate\fP, \fBvscode\fP, \fBvscode-insiders\fP, \fBvscodium\fP. The
alias will be replaced with a format string that is intended to work for the
corresponding application.
.sp
The following variables are available in the format string:
.sp
.TP 12
\fB{path}\fP
Required. This is replaced with a path to a matching file. The path is
guaranteed to be absolute and percent encoded such that it is valid to put into
a URI. Note that a path is guaranteed to start with a /.
.TP 12
\fB{host}\fP
Optional. This is replaced with your system's hostname. On Unix, this
corresponds to calling \fBgethostname\fP. On Windows, this corresponds to
calling \fBGetComputerNameExW\fP to fetch the system's "physical DNS hostname."
Alternatively, if \flag{hostname-bin} was provided, then the hostname returned
from the output of that program will be returned. If no hostname could be
found, then this variable is replaced with the empty string.
.TP 12
\fB{line}\fP
Optional. If appropriate, this is replaced with the line number of a match. If
no line number is available (for example, if \fB\-\-no\-line\-number\fP was
given), then it is automatically replaced with the value 1.
.TP 12
\fB{column}\fP
Optional, but requires the presence of \fB{line}\fP. If appropriate, this is
replaced with the column number of a match. If no column number is available
(for example, if \fB\-\-no\-column\fP was given), then it is automatically
replaced with the value 1.
.TP 12
\fB{wslprefix}\fP
Optional. This is a special value that is set to
\fBwsl$/\fP\fIWSL_DISTRO_NAME\fP, where \fIWSL_DISTRO_NAME\fP corresponds to
the value of the equivalent environment variable. If the system is not Unix
or if the \fIWSL_DISTRO_NAME\fP environment variable is not set, then this is
replaced with the empty string.
.PP
A format string may be empty. An empty format string is equivalent to the
\fBnone\fP alias. In this case, hyperlinks will be disabled.
.sp
At present, ripgrep does not enable hyperlinks by default. Users must opt into
them. If you aren't sure what format to use, try \fBdefault\fP.
.sp
Like colors, when ripgrep detects that stdout is not connected to a tty, then
hyperlinks are automatically disabled, regardless of the value of this flag.
Users can pass \fB\-\-color=always\fP to forcefully emit hyperlinks.
.sp
Note that hyperlinks are only written when a path is also in the output
and colors are enabled. To write hyperlinks without colors, you'll need to
configure ripgrep to not colorize anything without actually disabling all ANSI
escape codes completely:
.sp
.EX
    \-\-colors 'path:none' \\
    \-\-colors 'line:none' \\
    \-\-colors 'column:none' \\
    \-\-colors 'match:none'
.EE
.sp
ripgrep works this way because it treats the \flag{color} flag as a proxy for
whether ANSI escape codes should be used at all. This means that environment
variables like \fBNO_COLOR=1\fP and \fBTERM=dumb\fP not only disable colors,
but hyperlinks as well. Similarly, colors and hyperlinks are disabled when
ripgrep is not writing to a tty. (Unless one forces the issue by setting
\fB\-\-color=always\fP.)
.sp
If you're searching a file directly, for example:
.sp
.EX
    rg foo path/to/file
.EE
.sp
then hyperlinks will not be emitted since the path given does not appear
in the output. To make the path appear, and thus also a hyperlink, use the
\flag{with-filename} flag.
.sp
For more information on hyperlinks in terminal emulators, see:
https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let string = convert::str(&v)?;
        let format = string.parse().context("invalid hyperlink format")?;
        args.hyperlink_format = format;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hyperlink_format() {
    let parseformat = |format: &str| {
        format.parse::<grep::printer::HyperlinkFormat>().unwrap()
    };

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(parseformat("none"), args.hyperlink_format);

    let args = parse_low_raw(["--hyperlink-format", "default"]).unwrap();
    #[cfg(windows)]
    assert_eq!(parseformat("file://{path}"), args.hyperlink_format);
    #[cfg(not(windows))]
    assert_eq!(parseformat("file://{host}{path}"), args.hyperlink_format);

    let args = parse_low_raw(["--hyperlink-format", "file"]).unwrap();
    assert_eq!(parseformat("file://{host}{path}"), args.hyperlink_format);

    let args = parse_low_raw([
        "--hyperlink-format",
        "file",
        "--hyperlink-format=grep+",
    ])
    .unwrap();
    assert_eq!(parseformat("grep+://{path}:{line}"), args.hyperlink_format);

    let args =
        parse_low_raw(["--hyperlink-format", "file://{host}{path}#{line}"])
            .unwrap();
    assert_eq!(
        parseformat("file://{host}{path}#{line}"),
        args.hyperlink_format
    );

    let result = parse_low_raw(["--hyperlink-format", "file://heythere"]);
    assert!(result.is_err(), "{result:?}");
}

/// --iglob
#[derive(Debug)]
struct IGlob;

impl Flag for IGlob {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "iglob"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("GLOB")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Include/exclude paths case insensitively."
    }
    fn doc_long(&self) -> &'static str {
        r"
Include or exclude files and directories for searching that match the given
glob. This always overrides any other ignore logic. Multiple glob flags may
be used. Globbing rules match \fB.gitignore\fP globs. Precede a glob with a
\fB!\fP to exclude it. If multiple globs match a file or directory, the glob
given later in the command line takes precedence. Globs used via this flag are
matched case insensitively.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let glob = convert::string(v.unwrap_value())?;
        args.iglobs.push(glob);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_iglob() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<String>::new(), args.iglobs);

    let args = parse_low_raw(["--iglob", "foo"]).unwrap();
    assert_eq!(vec!["foo".to_string()], args.iglobs);

    let args = parse_low_raw(["--iglob=foo"]).unwrap();
    assert_eq!(vec!["foo".to_string()], args.iglobs);

    let args = parse_low_raw(["--iglob", "-foo"]).unwrap();
    assert_eq!(vec!["-foo".to_string()], args.iglobs);

    let args = parse_low_raw(["--iglob=-foo"]).unwrap();
    assert_eq!(vec!["-foo".to_string()], args.iglobs);
}

/// -i/--ignore-case
#[derive(Debug)]
struct IgnoreCase;

impl Flag for IgnoreCase {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'i')
    }
    fn name_long(&self) -> &'static str {
        "ignore-case"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Case insensitive search."
    }
    fn doc_long(&self) -> &'static str {
        r#"
When this flag is provided, all patterns will be searched case insensitively.
The case insensitivity rules used by ripgrep's default regex engine conform to
Unicode's "simple" case folding rules.
.sp
This is a global option that applies to all patterns given to ripgrep.
Individual patterns can still be matched case sensitively by using
inline regex flags. For example, \fB(?\-i)abc\fP will match \fBabc\fP
case sensitively even when this flag is used.
.sp
This flag overrides \flag{case-sensitive} and \flag{smart-case}.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "flag has no negation");
        args.case = CaseMode::Insensitive;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_ignore_case() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--ignore-case"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-i"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-i", "-s"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-s", "-i"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);
}

/// --ignore-file
#[derive(Debug)]
struct IgnoreFile;

impl Flag for IgnoreFile {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "ignore-file"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("PATH")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Specify additional ignore files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specifies a path to one or more \fBgitignore\fP formatted rules files.
These patterns are applied after the patterns found in \fB.gitignore\fP,
\fB.rgignore\fP and \fB.ignore\fP are applied and are matched relative to the
current working directory. Multiple additional ignore files can be specified
by using this flag repeatedly. When specifying multiple ignore files, earlier
files have lower precedence than later files.
.sp
If you are looking for a way to include or exclude files and directories
directly on the command line, then use \flag{glob} instead.
"
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Filename
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = PathBuf::from(v.unwrap_value());
        args.ignore_file.push(path);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_ignore_file() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<PathBuf>::new(), args.ignore_file);

    let args = parse_low_raw(["--ignore-file", "foo"]).unwrap();
    assert_eq!(vec![PathBuf::from("foo")], args.ignore_file);

    let args = parse_low_raw(["--ignore-file", "foo", "--ignore-file", "bar"])
        .unwrap();
    assert_eq!(
        vec![PathBuf::from("foo"), PathBuf::from("bar")],
        args.ignore_file
    );
}

/// --ignore-file-case-insensitive
#[derive(Debug)]
struct IgnoreFileCaseInsensitive;

impl Flag for IgnoreFileCaseInsensitive {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "ignore-file-case-insensitive"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-ignore-file-case-insensitive")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Process ignore files case insensitively."
    }
    fn doc_long(&self) -> &'static str {
        r"
Process ignore files (\fB.gitignore\fP, \fB.ignore\fP, etc.) case
insensitively. Note that this comes with a performance penalty and is most
useful on case insensitive file systems (such as Windows).
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.ignore_file_case_insensitive = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_ignore_file_case_insensitive() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.ignore_file_case_insensitive);

    let args = parse_low_raw(["--ignore-file-case-insensitive"]).unwrap();
    assert_eq!(true, args.ignore_file_case_insensitive);

    let args = parse_low_raw([
        "--ignore-file-case-insensitive",
        "--no-ignore-file-case-insensitive",
    ])
    .unwrap();
    assert_eq!(false, args.ignore_file_case_insensitive);

    let args = parse_low_raw([
        "--no-ignore-file-case-insensitive",
        "--ignore-file-case-insensitive",
    ])
    .unwrap();
    assert_eq!(true, args.ignore_file_case_insensitive);
}

/// --include-zero
#[derive(Debug)]
struct IncludeZero;

impl Flag for IncludeZero {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "include-zero"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-include-zero")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Include zero matches in summary output."
    }
    fn doc_long(&self) -> &'static str {
        r"
When used with \flag{count} or \flag{count-matches}, this causes ripgrep to
print the number of matches for each file even if there were zero matches. This
is disabled by default but can be enabled to make ripgrep behave more like
grep.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.include_zero = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_include_zero() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.include_zero);

    let args = parse_low_raw(["--include-zero"]).unwrap();
    assert_eq!(true, args.include_zero);

    let args = parse_low_raw(["--include-zero", "--no-include-zero"]).unwrap();
    assert_eq!(false, args.include_zero);
}

/// -v/--invert-match
#[derive(Debug)]
struct InvertMatch;

impl Flag for InvertMatch {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'v')
    }
    fn name_long(&self) -> &'static str {
        "invert-match"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-invert-match")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Invert matching."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag inverts matching. That is, instead of printing lines that match,
ripgrep will print lines that don't match.
.sp
Note that this only inverts line-by-line matching. For example, combining this
flag with \flag{files-with-matches} will emit files that contain any lines
that do not match the patterns given. That's not the same as, for example,
\flag{files-without-match}, which will emit files that do not contain any
matching lines.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.invert_match = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_invert_match() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.invert_match);

    let args = parse_low_raw(["--invert-match"]).unwrap();
    assert_eq!(true, args.invert_match);

    let args = parse_low_raw(["-v"]).unwrap();
    assert_eq!(true, args.invert_match);

    let args = parse_low_raw(["-v", "--no-invert-match"]).unwrap();
    assert_eq!(false, args.invert_match);
}

/// --json
#[derive(Debug)]
struct JSON;

impl Flag for JSON {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "json"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-json")
    }
    fn doc_category(&self) -> Category {
        Category::OutputModes
    }
    fn doc_short(&self) -> &'static str {
        r"Show search results in a JSON Lines format."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enable printing results in a JSON Lines format.
.sp
When this flag is provided, ripgrep will emit a sequence of messages, each
encoded as a JSON object, where there are five different message types:
.sp
.TP 12
\fBbegin\fP
A message that indicates a file is being searched and contains at least one
match.
.TP 12
\fBend\fP
A message the indicates a file is done being searched. This message also
include summary statistics about the search for a particular file.
.TP 12
\fBmatch\fP
A message that indicates a match was found. This includes the text and offsets
of the match.
.TP 12
\fBcontext\fP
A message that indicates a contextual line was found. This includes the text of
the line, along with any match information if the search was inverted.
.TP 12
\fBsummary\fP
The final message emitted by ripgrep that contains summary statistics about the
search across all files.
.PP
Since file paths or the contents of files are not guaranteed to be valid
UTF-8 and JSON itself must be representable by a Unicode encoding, ripgrep
will emit all data elements as objects with one of two keys: \fBtext\fP or
\fBbytes\fP. \fBtext\fP is a normal JSON string when the data is valid UTF-8
while \fBbytes\fP is the base64 encoded contents of the data.
.sp
The JSON Lines format is only supported for showing search results. It cannot
be used with other flags that emit other types of output, such as \flag{files},
\flag{files-with-matches}, \flag{files-without-match}, \flag{count} or
\flag{count-matches}. ripgrep will report an error if any of the aforementioned
flags are used in concert with \flag{json}.
.sp
Other flags that control aspects of the standard output such as
\flag{only-matching}, \flag{heading}, \flag{replace}, \flag{max-columns}, etc.,
have no effect when \flag{json} is set. However, enabling JSON output will
always implicitly and unconditionally enable \flag{stats}.
.sp
A more complete description of the JSON format used can be found here:
\fIhttps://docs.rs/grep-printer/*/grep_printer/struct.JSON.html\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        if v.unwrap_switch() {
            args.mode.update(Mode::Search(SearchMode::JSON));
        } else if matches!(args.mode, Mode::Search(SearchMode::JSON)) {
            // --no-json only reverts to the default mode if the mode is
            // JSON, otherwise it's a no-op.
            args.mode.update(Mode::Search(SearchMode::Standard));
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_json() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--json"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::JSON), args.mode);

    let args = parse_low_raw(["--json", "--no-json"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--json", "--files", "--no-json"]).unwrap();
    assert_eq!(Mode::Files, args.mode);

    let args = parse_low_raw(["--json", "-l", "--no-json"]).unwrap();
    assert_eq!(Mode::Search(SearchMode::FilesWithMatches), args.mode);
}

/// --line-buffered
#[derive(Debug)]
struct LineBuffered;

impl Flag for LineBuffered {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "line-buffered"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-line-buffered")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Force line buffering."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will always use line buffering. That is, whenever a
matching line is found, it will be flushed to stdout immediately. This is the
default when ripgrep's stdout is connected to a tty, but otherwise, ripgrep
will use block buffering, which is typically faster. This flag forces ripgrep
to use line buffering even if it would otherwise use block buffering. This is
typically useful in shell pipelines, for example:
.sp
.EX
    tail -f something.log | rg foo --line-buffered | rg bar
.EE
.sp
This overrides the \flag{block-buffered} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.buffer = if v.unwrap_switch() {
            BufferMode::Line
        } else {
            BufferMode::Auto
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_line_buffered() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(BufferMode::Auto, args.buffer);

    let args = parse_low_raw(["--line-buffered"]).unwrap();
    assert_eq!(BufferMode::Line, args.buffer);

    let args =
        parse_low_raw(["--line-buffered", "--no-line-buffered"]).unwrap();
    assert_eq!(BufferMode::Auto, args.buffer);

    let args = parse_low_raw(["--line-buffered", "--block-buffered"]).unwrap();
    assert_eq!(BufferMode::Block, args.buffer);
}

/// -n/--line-number
#[derive(Debug)]
struct LineNumber;

impl Flag for LineNumber {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'n')
    }
    fn name_long(&self) -> &'static str {
        "line-number"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show line numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show line numbers (1-based).
.sp
This is enabled by default when stdout is connected to a tty.
.sp
This flag can be disabled by \flag{no-line-number}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--line-number has no automatic negation");
        args.line_number = Some(true);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_line_number() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.line_number);

    let args = parse_low_raw(["--line-number"]).unwrap();
    assert_eq!(Some(true), args.line_number);

    let args = parse_low_raw(["-n"]).unwrap();
    assert_eq!(Some(true), args.line_number);

    let args = parse_low_raw(["-n", "--no-line-number"]).unwrap();
    assert_eq!(Some(false), args.line_number);
}

/// -N/--no-line-number
#[derive(Debug)]
struct LineNumberNo;

impl Flag for LineNumberNo {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'N')
    }
    fn name_long(&self) -> &'static str {
        "no-line-number"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Suppress line numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
Suppress line numbers.
.sp
Line numbers are off by default when stdout is not connected to a tty.
.sp
Line numbers can be forcefully turned on by \flag{line-number}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(
            v.unwrap_switch(),
            "--no-line-number has no automatic negation"
        );
        args.line_number = Some(false);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_line_number() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.line_number);

    let args = parse_low_raw(["--no-line-number"]).unwrap();
    assert_eq!(Some(false), args.line_number);

    let args = parse_low_raw(["-N"]).unwrap();
    assert_eq!(Some(false), args.line_number);

    let args = parse_low_raw(["-N", "--line-number"]).unwrap();
    assert_eq!(Some(true), args.line_number);
}

/// -x/--line-regexp
#[derive(Debug)]
struct LineRegexp;

impl Flag for LineRegexp {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'x')
    }
    fn name_long(&self) -> &'static str {
        "line-regexp"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Show matches surrounded by line boundaries."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will only show matches surrounded by line boundaries.
This is equivalent to surrounding every pattern with \fB^\fP and \fB$\fP. In
other words, this only prints lines where the entire line participates in a
match.
.sp
This overrides the \flag{word-regexp} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--line-regexp has no negation");
        args.boundary = Some(BoundaryMode::Line);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_line_regexp() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.boundary);

    let args = parse_low_raw(["--line-regexp"]).unwrap();
    assert_eq!(Some(BoundaryMode::Line), args.boundary);

    let args = parse_low_raw(["-x"]).unwrap();
    assert_eq!(Some(BoundaryMode::Line), args.boundary);
}

/// -M/--max-columns
#[derive(Debug)]
struct MaxColumns;

impl Flag for MaxColumns {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'M')
    }
    fn name_long(&self) -> &'static str {
        "max-columns"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Omit lines longer than this limit."
    }
    fn doc_long(&self) -> &'static str {
        r"
When given, ripgrep will omit lines longer than this limit in bytes. Instead of
printing long lines, only the number of matches in that line is printed.
.sp
When this flag is omitted or is set to \fB0\fP, then it has no effect.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let max = convert::u64(&v.unwrap_value())?;
        args.max_columns = if max == 0 { None } else { Some(max) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_columns() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.max_columns);

    let args = parse_low_raw(["--max-columns", "5"]).unwrap();
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["-M", "5"]).unwrap();
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["-M5"]).unwrap();
    assert_eq!(Some(5), args.max_columns);

    let args = parse_low_raw(["--max-columns", "5", "-M0"]).unwrap();
    assert_eq!(None, args.max_columns);
}

/// --max-columns-preview
#[derive(Debug)]
struct MaxColumnsPreview;

impl Flag for MaxColumnsPreview {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "max-columns-preview"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-max-columns-preview")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show preview for lines exceeding the limit."
    }
    fn doc_long(&self) -> &'static str {
        r"
Prints a preview for lines exceeding the configured max column limit.
.sp
When the \flag{max-columns} flag is used, ripgrep will by default completely
replace any line that is too long with a message indicating that a matching
line was removed. When this flag is combined with \flag{max-columns}, a preview
of the line (corresponding to the limit size) is shown instead, where the part
of the line exceeding the limit is not shown.
.sp
If the \flag{max-columns} flag is not set, then this has no effect.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_columns_preview = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_columns_preview() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.max_columns_preview);

    let args = parse_low_raw(["--max-columns-preview"]).unwrap();
    assert_eq!(true, args.max_columns_preview);

    let args =
        parse_low_raw(["--max-columns-preview", "--no-max-columns-preview"])
            .unwrap();
    assert_eq!(false, args.max_columns_preview);
}

/// -m/--max-count
#[derive(Debug)]
struct MaxCount;

impl Flag for MaxCount {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'm')
    }
    fn name_long(&self) -> &'static str {
        "max-count"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Limit the number of matching lines."
    }
    fn doc_long(&self) -> &'static str {
        r"
Limit the number of matching lines per file searched to \fINUM\fP.
.sp
Note that \fB0\fP is a legal value but not likely to be useful. When used,
ripgrep won't search anything.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_count = Some(convert::u64(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_count() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.max_count);

    let args = parse_low_raw(["--max-count", "5"]).unwrap();
    assert_eq!(Some(5), args.max_count);

    let args = parse_low_raw(["-m", "5"]).unwrap();
    assert_eq!(Some(5), args.max_count);

    let args = parse_low_raw(["-m", "5", "--max-count=10"]).unwrap();
    assert_eq!(Some(10), args.max_count);
    let args = parse_low_raw(["-m0"]).unwrap();
    assert_eq!(Some(0), args.max_count);
}

/// --max-depth
#[derive(Debug)]
struct MaxDepth;

impl Flag for MaxDepth {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'd')
    }
    fn name_long(&self) -> &'static str {
        "max-depth"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["maxdepth"]
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Descend at most NUM directories."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag limits the depth of directory traversal to \fINUM\fP levels beyond
the paths given. A value of \fB0\fP only searches the explicitly given paths
themselves.
.sp
For example, \fBrg --max-depth 0 \fP\fIdir/\fP is a no-op because \fIdir/\fP
will not be descended into. \fBrg --max-depth 1 \fP\fIdir/\fP will search only
the direct children of \fIdir\fP.
.sp
An alternative spelling for this flag is \fB\-\-maxdepth\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.max_depth = Some(convert::usize(&v.unwrap_value())?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_depth() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.max_depth);

    let args = parse_low_raw(["--max-depth", "5"]).unwrap();
    assert_eq!(Some(5), args.max_depth);

    let args = parse_low_raw(["-d", "5"]).unwrap();
    assert_eq!(Some(5), args.max_depth);

    let args = parse_low_raw(["--max-depth", "5", "--max-depth=10"]).unwrap();
    assert_eq!(Some(10), args.max_depth);

    let args = parse_low_raw(["--max-depth", "0"]).unwrap();
    assert_eq!(Some(0), args.max_depth);

    let args = parse_low_raw(["--maxdepth", "5"]).unwrap();
    assert_eq!(Some(5), args.max_depth);
}

/// --max-filesize
#[derive(Debug)]
struct MaxFilesize;

impl Flag for MaxFilesize {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "max-filesize"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Ignore files larger than NUM in size."
    }
    fn doc_long(&self) -> &'static str {
        r"
Ignore files larger than \fINUM\fP in size. This does not apply to directories.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
.sp
Examples: \fB\-\-max-filesize 50K\fP or \fB\-\-max\-filesize 80M\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.max_filesize = Some(convert::human_readable_u64(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_max_filesize() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.max_filesize);

    let args = parse_low_raw(["--max-filesize", "1024"]).unwrap();
    assert_eq!(Some(1024), args.max_filesize);

    let args = parse_low_raw(["--max-filesize", "1K"]).unwrap();
    assert_eq!(Some(1024), args.max_filesize);

    let args =
        parse_low_raw(["--max-filesize", "1K", "--max-filesize=1M"]).unwrap();
    assert_eq!(Some(1024 * 1024), args.max_filesize);
}

/// --mmap
#[derive(Debug)]
struct Mmap;

impl Flag for Mmap {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "mmap"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-mmap")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search with memory maps when possible."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will search using memory maps when possible. This is
enabled by default when ripgrep thinks it will be faster.
.sp
Memory map searching cannot be used in all circumstances. For example, when
searching virtual files or streams likes \fBstdin\fP. In such cases, memory
maps will not be used even when this flag is enabled.
.sp
Note that ripgrep may abort unexpectedly when memory maps are used if it
searches a file that is simultaneously truncated. Users can opt out of this
possibility by disabling memory maps.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.mmap = if v.unwrap_switch() {
            MmapMode::AlwaysTryMmap
        } else {
            MmapMode::Never
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_mmap() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(MmapMode::Auto, args.mmap);

    let args = parse_low_raw(["--mmap"]).unwrap();
    assert_eq!(MmapMode::AlwaysTryMmap, args.mmap);

    let args = parse_low_raw(["--no-mmap"]).unwrap();
    assert_eq!(MmapMode::Never, args.mmap);

    let args = parse_low_raw(["--mmap", "--no-mmap"]).unwrap();
    assert_eq!(MmapMode::Never, args.mmap);

    let args = parse_low_raw(["--no-mmap", "--mmap"]).unwrap();
    assert_eq!(MmapMode::AlwaysTryMmap, args.mmap);
}

/// -U/--multiline
#[derive(Debug)]
struct Multiline;

impl Flag for Multiline {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'U')
    }
    fn name_long(&self) -> &'static str {
        "multiline"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-multiline")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Enable searching across multiple lines."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag enable searching across multiple lines.
.sp
When multiline mode is enabled, ripgrep will lift the restriction that a
match cannot include a line terminator. For example, when multiline mode
is not enabled (the default), then the regex \fB\\p{any}\fP will match any
Unicode codepoint other than \fB\\n\fP. Similarly, the regex \fB\\n\fP is
explicitly forbidden, and if you try to use it, ripgrep will return an error.
However, when multiline mode is enabled, \fB\\p{any}\fP will match any Unicode
codepoint, including \fB\\n\fP, and regexes like \fB\\n\fP are permitted.
.sp
An important caveat is that multiline mode does not change the match semantics
of \fB.\fP. Namely, in most regex matchers, a \fB.\fP will by default match any
character other than \fB\\n\fP, and this is true in ripgrep as well. In order
to make \fB.\fP match \fB\\n\fP, you must enable the "dot all" flag inside the
regex. For example, both \fB(?s).\fP and \fB(?s:.)\fP have the same semantics,
where \fB.\fP will match any character, including \fB\\n\fP. Alternatively, the
\flag{multiline-dotall} flag may be passed to make the "dot all" behavior the
default. This flag only applies when multiline search is enabled.
.sp
There is no limit on the number of the lines that a single match can span.
.sp
\fBWARNING\fP: Because of how the underlying regex engine works, multiline
searches may be slower than normal line-oriented searches, and they may also
use more memory. In particular, when multiline mode is enabled, ripgrep
requires that each file it searches is laid out contiguously in memory (either
by reading it onto the heap or by memory-mapping it). Things that cannot be
memory-mapped (such as \fBstdin\fP) will be consumed until EOF before searching
can begin. In general, ripgrep will only do these things when necessary.
Specifically, if the \flag{multiline} flag is provided but the regex does
not contain patterns that would match \fB\\n\fP characters, then ripgrep
will automatically avoid reading each file into memory before searching it.
Nevertheless, if you only care about matches spanning at most one line, then it
is always better to disable multiline mode.
.sp
This overrides the \flag{stop-on-nonmatch} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.multiline = v.unwrap_switch();
        if args.multiline {
            args.stop_on_nonmatch = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_multiline() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.multiline);

    let args = parse_low_raw(["--multiline"]).unwrap();
    assert_eq!(true, args.multiline);

    let args = parse_low_raw(["-U"]).unwrap();
    assert_eq!(true, args.multiline);

    let args = parse_low_raw(["-U", "--no-multiline"]).unwrap();
    assert_eq!(false, args.multiline);
}

/// --multiline-dotall
#[derive(Debug)]
struct MultilineDotall;

impl Flag for MultilineDotall {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "multiline-dotall"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-multiline-dotall")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Make '.' match line terminators."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag enables "dot all" mode in all regex patterns. This causes \fB.\fP to
match line terminators when multiline searching is enabled. This flag has no
effect if multiline searching isn't enabled with the \flag{multiline} flag.
.sp
Normally, a \fB.\fP will match any character except line terminators. While
this behavior typically isn't relevant for line-oriented matching (since
matches can span at most one line), this can be useful when searching with the
\flag{multiline} flag. By default, multiline mode runs without "dot all" mode
enabled.
.sp
This flag is generally intended to be used in an alias or your ripgrep config
file if you prefer "dot all" semantics by default. Note that regardless of
whether this flag is used, "dot all" semantics can still be controlled via
inline flags in the regex pattern itself, e.g., \fB(?s:.)\fP always enables
"dot all" whereas \fB(?-s:.)\fP always disables "dot all". Moreover, you
can use character classes like \fB\\p{any}\fP to match any Unicode codepoint
regardless of whether "dot all" mode is enabled or not.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.multiline_dotall = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_multiline_dotall() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.multiline_dotall);

    let args = parse_low_raw(["--multiline-dotall"]).unwrap();
    assert_eq!(true, args.multiline_dotall);

    let args = parse_low_raw(["--multiline-dotall", "--no-multiline-dotall"])
        .unwrap();
    assert_eq!(false, args.multiline_dotall);
}

/// --no-config
#[derive(Debug)]
struct NoConfig;

impl Flag for NoConfig {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-config"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Never read configuration files."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, ripgrep will never read configuration files. When this flag is
present, ripgrep will not respect the \fBRIPGREP_CONFIG_PATH\fP environment
variable.
.sp
If ripgrep ever grows a feature to automatically read configuration files in
pre-defined locations, then this flag will also disable that behavior as well.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--no-config has no negation");
        args.no_config = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_config() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_config);

    let args = parse_low_raw(["--no-config"]).unwrap();
    assert_eq!(true, args.no_config);
}

/// --no-ignore
#[derive(Debug)]
struct NoIgnore;

impl Flag for NoIgnore {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use ignore files."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, ignore files such as \fB.gitignore\fP, \fB.ignore\fP and
\fB.rgignore\fP will not be respected. This implies \flag{no-ignore-dot},
\flag{no-ignore-exclude}, \flag{no-ignore-global}, \flag{no-ignore-parent} and
\flag{no-ignore-vcs}.
.sp
This does not imply \flag{no-ignore-files}, since \flag{ignore-file} is
specified explicitly as a command line argument.
.sp
When given only once, the \flag{unrestricted} flag is identical in
behavior to this flag and can be considered an alias. However, subsequent
\flag{unrestricted} flags have additional effects.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let yes = v.unwrap_switch();
        args.no_ignore_dot = yes;
        args.no_ignore_exclude = yes;
        args.no_ignore_global = yes;
        args.no_ignore_parent = yes;
        args.no_ignore_vcs = yes;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_dot);
    assert_eq!(false, args.no_ignore_exclude);
    assert_eq!(false, args.no_ignore_global);
    assert_eq!(false, args.no_ignore_parent);
    assert_eq!(false, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore"]).unwrap();
    assert_eq!(true, args.no_ignore_dot);
    assert_eq!(true, args.no_ignore_exclude);
    assert_eq!(true, args.no_ignore_global);
    assert_eq!(true, args.no_ignore_parent);
    assert_eq!(true, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore", "--ignore"]).unwrap();
    assert_eq!(false, args.no_ignore_dot);
    assert_eq!(false, args.no_ignore_exclude);
    assert_eq!(false, args.no_ignore_global);
    assert_eq!(false, args.no_ignore_parent);
    assert_eq!(false, args.no_ignore_vcs);
}

/// --no-ignore-dot
#[derive(Debug)]
struct NoIgnoreDot;

impl Flag for NoIgnoreDot {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-dot"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-dot")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use .ignore or .rgignore files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Don't respect filter rules from \fB.ignore\fP or \fB.rgignore\fP files.
.sp
This does not impact whether ripgrep will ignore files and directories whose
names begin with a dot. For that, see the \flag{hidden} flag. This flag also
does not impact whether filter rules from \fB.gitignore\fP files are respected.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_dot = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_dot() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_dot);

    let args = parse_low_raw(["--no-ignore-dot"]).unwrap();
    assert_eq!(true, args.no_ignore_dot);

    let args = parse_low_raw(["--no-ignore-dot", "--ignore-dot"]).unwrap();
    assert_eq!(false, args.no_ignore_dot);
}

/// --no-ignore-exclude
#[derive(Debug)]
struct NoIgnoreExclude;

impl Flag for NoIgnoreExclude {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-exclude"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-exclude")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use local exclusion files."
    }
    fn doc_long(&self) -> &'static str {
        r"
Don't respect filter rules from files that are manually configured for the repository.
For example, this includes \fBgit\fP's \fB.git/info/exclude\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_exclude = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_exclude() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_exclude);

    let args = parse_low_raw(["--no-ignore-exclude"]).unwrap();
    assert_eq!(true, args.no_ignore_exclude);

    let args =
        parse_low_raw(["--no-ignore-exclude", "--ignore-exclude"]).unwrap();
    assert_eq!(false, args.no_ignore_exclude);
}

/// --no-ignore-files
#[derive(Debug)]
struct NoIgnoreFiles;

impl Flag for NoIgnoreFiles {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-files"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-files")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use --ignore-file arguments."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, any \flag{ignore-file} flags, even ones that come after this flag,
are ignored.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_files = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_files() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_files);

    let args = parse_low_raw(["--no-ignore-files"]).unwrap();
    assert_eq!(true, args.no_ignore_files);

    let args = parse_low_raw(["--no-ignore-files", "--ignore-files"]).unwrap();
    assert_eq!(false, args.no_ignore_files);
}

/// --no-ignore-global
#[derive(Debug)]
struct NoIgnoreGlobal;

impl Flag for NoIgnoreGlobal {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-global"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-global")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use global ignore files."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Don't respect filter rules from ignore files that come from "global" sources
such as \fBgit\fP's \fBcore.excludesFile\fP configuration option (which
defaults to \fB$HOME/.config/git/ignore\fP).
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_global = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_global() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_global);

    let args = parse_low_raw(["--no-ignore-global"]).unwrap();
    assert_eq!(true, args.no_ignore_global);

    let args =
        parse_low_raw(["--no-ignore-global", "--ignore-global"]).unwrap();
    assert_eq!(false, args.no_ignore_global);
}

/// --no-ignore-messages
#[derive(Debug)]
struct NoIgnoreMessages;

impl Flag for NoIgnoreMessages {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-messages"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-messages")
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Suppress gitignore parse error messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is enabled, all error messages related to parsing ignore files
are suppressed. By default, error messages are printed to stderr. In cases
where these errors are expected, this flag can be used to avoid seeing the
noise produced by the messages.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_messages = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_messages() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_messages);

    let args = parse_low_raw(["--no-ignore-messages"]).unwrap();
    assert_eq!(true, args.no_ignore_messages);

    let args =
        parse_low_raw(["--no-ignore-messages", "--ignore-messages"]).unwrap();
    assert_eq!(false, args.no_ignore_messages);
}

/// --no-ignore-parent
#[derive(Debug)]
struct NoIgnoreParent;

impl Flag for NoIgnoreParent {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-parent"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-parent")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use ignore files in parent directories."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is set, filter rules from ignore files found in parent
directories are not respected. By default, ripgrep will ascend the parent
directories of the current working directory to look for any applicable ignore
files that should be applied. In some cases this may not be desirable.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_parent = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_parent() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_parent);

    let args = parse_low_raw(["--no-ignore-parent"]).unwrap();
    assert_eq!(true, args.no_ignore_parent);

    let args =
        parse_low_raw(["--no-ignore-parent", "--ignore-parent"]).unwrap();
    assert_eq!(false, args.no_ignore_parent);
}

/// --no-ignore-vcs
#[derive(Debug)]
struct NoIgnoreVcs;

impl Flag for NoIgnoreVcs {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-ignore-vcs"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("ignore-vcs")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Don't use ignore files from source control."
    }
    fn doc_long(&self) -> &'static str {
        r"
When given, filter rules from source control ignore files (e.g., \fB.gitignore\fP)
are not respected. By default, ripgrep respects \fBgit\fP's ignore rules for
automatic filtering. In some cases, it may not be desirable to respect the
source control's ignore rules and instead only respect rules in \fB.ignore\fP
or \fB.rgignore\fP.
.sp
This flag implies \flag{no-ignore-parent} for source control ignore files as
well.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_ignore_vcs = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_ignore_vcs() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore-vcs"]).unwrap();
    assert_eq!(true, args.no_ignore_vcs);

    let args = parse_low_raw(["--no-ignore-vcs", "--ignore-vcs"]).unwrap();
    assert_eq!(false, args.no_ignore_vcs);
}

/// --no-messages
#[derive(Debug)]
struct NoMessages;

impl Flag for NoMessages {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-messages"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("messages")
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Suppress some error messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag suppresses some error messages. Specifically, messages related to
the failed opening and reading of files. Error messages related to the syntax
of the pattern are still shown.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_messages = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_messages() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_messages);

    let args = parse_low_raw(["--no-messages"]).unwrap();
    assert_eq!(true, args.no_messages);

    let args = parse_low_raw(["--no-messages", "--messages"]).unwrap();
    assert_eq!(false, args.no_messages);
}

/// --no-pcre2-unicode
#[derive(Debug)]
struct NoPcre2Unicode;

impl Flag for NoPcre2Unicode {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-pcre2-unicode"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("pcre2-unicode")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"(DEPRECATED) Disable Unicode mode for PCRE2."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \flag{no-unicode} instead.
.sp
Note that Unicode mode is enabled by default.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_unicode = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_pcre2_unicode() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-pcre2-unicode"]).unwrap();
    assert_eq!(true, args.no_unicode);

    let args =
        parse_low_raw(["--no-pcre2-unicode", "--pcre2-unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);
}

/// --no-require-git
#[derive(Debug)]
struct NoRequireGit;

impl Flag for NoRequireGit {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-require-git"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("require-git")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Use .gitignore outside of git repositories."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is given, source control ignore files such as \fB.gitignore\fP
are respected even if no \fBgit\fP repository is present.
.sp
By default, ripgrep will only respect filter rules from source control ignore
files when ripgrep detects that the search is executed inside a source control
repository. For example, when a \fB.git\fP directory is observed.
.sp
This flag relaxes the default restriction. For example, it might be useful when
the contents of a \fBgit\fP repository are stored or copied somewhere, but
where the repository state is absent.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_require_git = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_require_git() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_require_git);

    let args = parse_low_raw(["--no-require-git"]).unwrap();
    assert_eq!(true, args.no_require_git);

    let args = parse_low_raw(["--no-require-git", "--require-git"]).unwrap();
    assert_eq!(false, args.no_require_git);
}

/// --no-unicode
#[derive(Debug)]
struct NoUnicode;

impl Flag for NoUnicode {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-unicode"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("unicode")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Disable Unicode mode."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag disables Unicode mode for all patterns given to ripgrep.
.sp
By default, ripgrep will enable "Unicode mode" in all of its regexes. This has
a number of consequences:
.sp
.IP \(bu 3n
\fB.\fP will only match valid UTF-8 encoded Unicode scalar values.
.sp
.IP \(bu 3n
Classes like \fB\\w\fP, \fB\\s\fP, \fB\\d\fP are all Unicode aware and much
bigger than their ASCII only versions.
.sp
.IP \(bu 3n
Case insensitive matching will use Unicode case folding.
.sp
.IP \(bu 3n
A large array of classes like \fB\\p{Emoji}\fP are available. (Although the
specific set of classes available varies based on the regex engine. In general,
the default regex engine has more classes available to it.)
.sp
.IP \(bu 3n
Word boundaries (\fB\\b\fP and \fB\\B\fP) use the Unicode definition of a word
character.
.PP
In some cases it can be desirable to turn these things off. This flag will do
exactly that. For example, Unicode mode can sometimes have a negative impact
on performance, especially when things like \fB\\w\fP are used frequently
(including via bounded repetitions like \fB\\w{100}\fP) when only their ASCII
interpretation is needed.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_unicode = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_unicode() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-unicode"]).unwrap();
    assert_eq!(true, args.no_unicode);

    let args = parse_low_raw(["--no-unicode", "--unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-unicode", "--pcre2-unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-pcre2-unicode", "--unicode"]).unwrap();
    assert_eq!(false, args.no_unicode);
}

/// -0/--null
#[derive(Debug)]
struct Null;

impl Flag for Null {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'0')
    }
    fn name_long(&self) -> &'static str {
        "null"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print a NUL byte after file paths."
    }
    fn doc_long(&self) -> &'static str {
        r"
Whenever a file path is printed, follow it with a \fBNUL\fP byte. This includes
printing file paths before matches, and when printing a list of matching files
such as with \flag{count}, \flag{files-with-matches} and \flag{files}. This
option is useful for use with \fBxargs\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--null has no negation");
        args.null = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_null() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.null);

    let args = parse_low_raw(["--null"]).unwrap();
    assert_eq!(true, args.null);

    let args = parse_low_raw(["-0"]).unwrap();
    assert_eq!(true, args.null);
}

/// --null-data
#[derive(Debug)]
struct NullData;

impl Flag for NullData {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "null-data"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Use NUL as a line terminator."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this flag causes ripgrep to use \fBNUL\fP as a line terminator instead
of the default of \fP\\n\fP.
.sp
This is useful when searching large binary files that would otherwise have
very long lines if \fB\\n\fP were used as the line terminator. In particular,
ripgrep requires that, at a minimum, each line must fit into memory. Using
\fBNUL\fP instead can be a useful stopgap to keep memory requirements low and
avoid OOM (out of memory) conditions.
.sp
This is also useful for processing NUL delimited data, such as that emitted
when using ripgrep's \flag{null} flag or \fBfind\fP's \fB\-\-print0\fP flag.
.sp
Using this flag implies \flag{text}. It also overrides \flag{crlf}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--null-data has no negation");
        args.crlf = false;
        args.null_data = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_null_data() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.null_data);

    let args = parse_low_raw(["--null-data"]).unwrap();
    assert_eq!(true, args.null_data);

    let args = parse_low_raw(["--null-data", "--crlf"]).unwrap();
    assert_eq!(false, args.null_data);
    assert_eq!(true, args.crlf);

    let args = parse_low_raw(["--crlf", "--null-data"]).unwrap();
    assert_eq!(true, args.null_data);
    assert_eq!(false, args.crlf);

    let args = parse_low_raw(["--null-data", "--no-crlf"]).unwrap();
    assert_eq!(true, args.null_data);
    assert_eq!(false, args.crlf);
}

/// --one-file-system
#[derive(Debug)]
struct OneFileSystem;

impl Flag for OneFileSystem {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "one-file-system"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-one-file-system")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Skip directories on other file systems."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will not cross file system boundaries relative to where
the search started from.
.sp
Note that this applies to each path argument given to ripgrep. For example, in
the command
.sp
.EX
    rg \-\-one\-file\-system /foo/bar /quux/baz
.EE
.sp
ripgrep will search both \fI/foo/bar\fP and \fI/quux/baz\fP even if they are
on different file systems, but will not cross a file system boundary when
traversing each path's directory tree.
.sp
This is similar to \fBfind\fP's \fB\-xdev\fP or \fB\-mount\fP flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.one_file_system = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_one_file_system() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.one_file_system);

    let args = parse_low_raw(["--one-file-system"]).unwrap();
    assert_eq!(true, args.one_file_system);

    let args =
        parse_low_raw(["--one-file-system", "--no-one-file-system"]).unwrap();
    assert_eq!(false, args.one_file_system);
}

/// -o/--only-matching
#[derive(Debug)]
struct OnlyMatching;

impl Flag for OnlyMatching {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'o')
    }
    fn name_long(&self) -> &'static str {
        "only-matching"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print only matched parts of a line."
    }
    fn doc_long(&self) -> &'static str {
        r"
Print only the matched (non-empty) parts of a matching line, with each such
part on a separate output line.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--only-matching does not have a negation");
        args.only_matching = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_only_matching() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.only_matching);

    let args = parse_low_raw(["--only-matching"]).unwrap();
    assert_eq!(true, args.only_matching);

    let args = parse_low_raw(["-o"]).unwrap();
    assert_eq!(true, args.only_matching);
}

/// --path-separator
#[derive(Debug)]
struct PathSeparator;

impl Flag for PathSeparator {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "path-separator"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SEPARATOR")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the path separator for printing paths."
    }
    fn doc_long(&self) -> &'static str {
        r"
Set the path separator to use when printing file paths. This defaults to your
platform's path separator, which is \fB/\fP on Unix and \fB\\\fP on Windows.
This flag is intended for overriding the default when the environment demands
it (e.g., cygwin). A path separator is limited to a single byte.
.sp
Setting this flag to an empty string reverts it to its default behavior. That
is, the path separator is automatically chosen based on the environment.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let s = convert::string(v.unwrap_value())?;
        let raw = Vec::unescape_bytes(&s);
        args.path_separator = if raw.is_empty() {
            None
        } else if raw.len() == 1 {
            Some(raw[0])
        } else {
            anyhow::bail!(
                "A path separator must be exactly one byte, but \
                 the given separator is {len} bytes: {sep}\n\
                 In some shells on Windows '/' is automatically \
                 expanded. Use '//' instead.",
                len = raw.len(),
                sep = s,
            )
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_path_separator() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.path_separator);

    let args = parse_low_raw(["--path-separator", "/"]).unwrap();
    assert_eq!(Some(b'/'), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\"]).unwrap();
    assert_eq!(Some(b'\\'), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\x00"]).unwrap();
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", r"\0"]).unwrap();
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", "\x00"]).unwrap();
    assert_eq!(Some(0), args.path_separator);

    let args = parse_low_raw(["--path-separator", "\0"]).unwrap();
    assert_eq!(Some(0), args.path_separator);

    let args =
        parse_low_raw(["--path-separator", r"\x00", "--path-separator=/"])
            .unwrap();
    assert_eq!(Some(b'/'), args.path_separator);

    let result = parse_low_raw(["--path-separator", "foo"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--path-separator", r"\\x00"]);
    assert!(result.is_err(), "{result:?}");
}

/// --passthru
#[derive(Debug)]
struct Passthru;

impl Flag for Passthru {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "passthru"
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["passthrough"]
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print both matching and non-matching lines."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Print both matching and non-matching lines.
.sp
Another way to achieve a similar effect is by modifying your pattern to match
the empty string. For example, if you are searching using \fBrg\fP \fIfoo\fP,
then using \fBrg\fP \fB'^|\fP\fIfoo\fP\fB'\fP instead will emit every line in
every file searched, but only occurrences of \fIfoo\fP will be highlighted.
This flag enables the same behavior without needing to modify the pattern.
.sp
An alternative spelling for this flag is \fB\-\-passthrough\fP.
.sp
This overrides the \flag{context}, \flag{after-context} and
\flag{before-context} flags.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--passthru has no negation");
        args.context = ContextMode::Passthru;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_passthru() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(ContextMode::default(), args.context);

    let args = parse_low_raw(["--passthru"]).unwrap();
    assert_eq!(ContextMode::Passthru, args.context);

    let args = parse_low_raw(["--passthrough"]).unwrap();
    assert_eq!(ContextMode::Passthru, args.context);
}

/// -P/--pcre2
#[derive(Debug)]
struct PCRE2;

impl Flag for PCRE2 {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'P')
    }
    fn name_long(&self) -> &'static str {
        "pcre2"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-pcre2")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Enable PCRE2 matching."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is present, ripgrep will use the PCRE2 regex engine instead of
its default regex engine.
.sp
This is generally useful when you want to use features such as look-around
or backreferences.
.sp
Using this flag is the same as passing \fB\-\-engine=pcre2\fP. Users may
instead elect to use \fB\-\-engine=auto\fP to ask ripgrep to automatically
select the right regex engine based on the patterns given. This flag and the
\flag{engine} flag override one another.
.sp
Note that PCRE2 is an optional ripgrep feature. If PCRE2 wasn't included in
your build of ripgrep, then using this flag will result in ripgrep printing
an error message and exiting. PCRE2 may also have worse user experience in
some cases, since it has fewer introspection APIs than ripgrep's default
regex engine. For example, if you use a \fB\\n\fP in a PCRE2 regex without
the \flag{multiline} flag, then ripgrep will silently fail to match anything
instead of reporting an error immediately (like it does with the default regex
engine).
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.engine = if v.unwrap_switch() {
            EngineChoice::PCRE2
        } else {
            EngineChoice::Default
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pcre2() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--pcre2"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["-P"]).unwrap();
    assert_eq!(EngineChoice::PCRE2, args.engine);

    let args = parse_low_raw(["-P", "--no-pcre2"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["--engine=auto", "-P", "--no-pcre2"]).unwrap();
    assert_eq!(EngineChoice::Default, args.engine);

    let args = parse_low_raw(["-P", "--engine=auto"]).unwrap();
    assert_eq!(EngineChoice::Auto, args.engine);
}

/// --pcre2-version
#[derive(Debug)]
struct PCRE2Version;

impl Flag for PCRE2Version {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "pcre2-version"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Print the version of PCRE2 that ripgrep uses."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is present, ripgrep will print the version of PCRE2 in use,
along with other information, and then exit. If PCRE2 is not available, then
ripgrep will print an error message and exit with an error code.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--pcre2-version has no negation");
        args.special = Some(SpecialMode::VersionPCRE2);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pcre2_version() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.special);

    let args = parse_low_raw(["--pcre2-version"]).unwrap();
    assert_eq!(Some(SpecialMode::VersionPCRE2), args.special);
}

/// --pre
#[derive(Debug)]
struct Pre;

impl Flag for Pre {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "pre"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-pre")
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COMMAND")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"Search output of COMMAND for each PATH."
    }
    fn doc_long(&self) -> &'static str {
        r#"
For each input \fIPATH\fP, this flag causes ripgrep to search the standard
output of \fICOMMAND\fP \fIPATH\fP instead of the contents of \fIPATH\fP.
This option expects the \fICOMMAND\fP program to either be a path or to be
available in your \fBPATH\fP. Either an empty string \fICOMMAND\fP or the
\fB\-\-no\-pre\fP flag will disable this behavior.
.sp
.TP 12
\fBWARNING\fP
When this flag is set, ripgrep will unconditionally spawn a process for every
file that is searched. Therefore, this can incur an unnecessarily large
performance penalty if you don't otherwise need the flexibility offered by this
flag. One possible mitigation to this is to use the \flag{pre-glob} flag to
limit which files a preprocessor is run with.
.PP
A preprocessor is not run when ripgrep is searching stdin.
.sp
When searching over sets of files that may require one of several
preprocessors, \fICOMMAND\fP should be a wrapper program which first classifies
\fIPATH\fP based on magic numbers/content or based on the \fIPATH\fP name and
then dispatches to an appropriate preprocessor. Each \fICOMMAND\fP also has its
standard input connected to \fIPATH\fP for convenience.
.sp
For example, a shell script for \fICOMMAND\fP might look like:
.sp
.EX
    case "$1" in
    *.pdf)
        exec pdftotext "$1" -
        ;;
    *)
        case $(file "$1") in
        *Zstandard*)
            exec pzstd -cdq
            ;;
        *)
            exec cat
            ;;
        esac
        ;;
    esac
.EE
.sp
The above script uses \fBpdftotext\fP to convert a PDF file to plain text. For
all other files, the script uses the \fBfile\fP utility to sniff the type of
the file based on its contents. If it is a compressed file in the Zstandard
format, then \fBpzstd\fP is used to decompress the contents to stdout.
.sp
This overrides the \flag{search-zip} flag.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Executable
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = match v {
            FlagValue::Value(v) => PathBuf::from(v),
            FlagValue::Switch(yes) => {
                assert!(!yes, "there is no affirmative switch for --pre");
                args.pre = None;
                return Ok(());
            }
        };
        args.pre = if path.as_os_str().is_empty() { None } else { Some(path) };
        if args.pre.is_some() {
            args.search_zip = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pre() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar"]).unwrap();
    assert_eq!(Some(PathBuf::from("foo/bar")), args.pre);

    let args = parse_low_raw(["--pre", ""]).unwrap();
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar", "--pre", ""]).unwrap();
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar", "--pre="]).unwrap();
    assert_eq!(None, args.pre);

    let args = parse_low_raw(["--pre", "foo/bar", "--no-pre"]).unwrap();
    assert_eq!(None, args.pre);
}

/// --pre-glob
#[derive(Debug)]
struct PreGlob;

impl Flag for PreGlob {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "pre-glob"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("GLOB")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"Include or exclude files from a preprocessor."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag works in conjunction with the \flag{pre} flag. Namely, when one or
more \flag{pre-glob} flags are given, then only files that match the given set
of globs will be handed to the command specified by the \flag{pre} flag. Any
non-matching files will be searched without using the preprocessor command.
.sp
This flag is useful when searching many files with the \flag{pre} flag.
Namely, it provides the ability to avoid process overhead for files that
don't need preprocessing. For example, given the following shell script,
\fIpre-pdftotext\fP:
.sp
.EX
    #!/bin/sh
    pdftotext "$1" -
.EE
.sp
then it is possible to use \fB\-\-pre\fP \fIpre-pdftotext\fP \fB--pre-glob
'\fP\fI*.pdf\fP\fB'\fP to make it so ripgrep only executes the
\fIpre-pdftotext\fP command on files with a \fI.pdf\fP extension.
.sp
Multiple \flag{pre-glob} flags may be used. Globbing rules match
\fBgitignore\fP globs. Precede a glob with a \fB!\fP to exclude it.
.sp
This flag has no effect if the \flag{pre} flag is not used.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let glob = convert::string(v.unwrap_value())?;
        args.pre_glob.push(glob);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pre_glob() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<String>::new(), args.pre_glob);

    let args = parse_low_raw(["--pre-glob", "*.pdf"]).unwrap();
    assert_eq!(vec!["*.pdf".to_string()], args.pre_glob);

    let args =
        parse_low_raw(["--pre-glob", "*.pdf", "--pre-glob=foo"]).unwrap();
    assert_eq!(vec!["*.pdf".to_string(), "foo".to_string()], args.pre_glob);
}

/// -p/--pretty
#[derive(Debug)]
struct Pretty;

impl Flag for Pretty {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'p')
    }
    fn name_long(&self) -> &'static str {
        "pretty"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Alias for colors, headings and line numbers."
    }
    fn doc_long(&self) -> &'static str {
        r"
This is a convenience alias for \fB\-\-color=always \-\-heading
\-\-line\-number\fP. This flag is useful when you still want pretty output even
if you're piping ripgrep to another program or file. For example: \fBrg -p
\fP\fIfoo\fP \fB| less -R\fP.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--pretty has no negation");
        args.color = ColorChoice::Always;
        args.heading = Some(true);
        args.line_number = Some(true);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pretty() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(ColorChoice::Auto, args.color);
    assert_eq!(None, args.heading);
    assert_eq!(None, args.line_number);

    let args = parse_low_raw(["--pretty"]).unwrap();
    assert_eq!(ColorChoice::Always, args.color);
    assert_eq!(Some(true), args.heading);
    assert_eq!(Some(true), args.line_number);

    let args = parse_low_raw(["-p"]).unwrap();
    assert_eq!(ColorChoice::Always, args.color);
    assert_eq!(Some(true), args.heading);
    assert_eq!(Some(true), args.line_number);
}

/// -q/--quiet
#[derive(Debug)]
struct Quiet;

impl Flag for Quiet {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'q')
    }
    fn name_long(&self) -> &'static str {
        "quiet"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Do not print anything to stdout."
    }
    fn doc_long(&self) -> &'static str {
        r"
Do not print anything to stdout. If a match is found in a file, then ripgrep
will stop searching. This is useful when ripgrep is used only for its exit code
(which will be an error code if no matches are found).
.sp
When \flag{files} is used, ripgrep will stop finding files after finding the
first file that does not match any ignore rules.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--quiet has no negation");
        args.quiet = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_quiet() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.quiet);

    let args = parse_low_raw(["--quiet"]).unwrap();
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q"]).unwrap();
    assert_eq!(true, args.quiet);

    // flags like -l and --json cannot override -q, regardless of order
    let args = parse_low_raw(["-q", "--json"]).unwrap();
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--files-with-matches"]).unwrap();
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--files-without-match"]).unwrap();
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--count"]).unwrap();
    assert_eq!(true, args.quiet);

    let args = parse_low_raw(["-q", "--count-matches"]).unwrap();
    assert_eq!(true, args.quiet);
}

/// --regex-size-limit
#[derive(Debug)]
struct RegexSizeLimit;

impl Flag for RegexSizeLimit {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "regex-size-limit"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM+SUFFIX?")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"The size limit of the compiled regex."
    }
    fn doc_long(&self) -> &'static str {
        r"
The size limit of the compiled regex, where the compiled regex generally
corresponds to a single object in memory that can match all of the patterns
provided to ripgrep. The default limit is generous enough that most reasonable
patterns (or even a small number of them) should fit.
.sp
This useful to change when you explicitly want to let ripgrep spend potentially
much more time and/or memory building a regex matcher.
.sp
The input format accepts suffixes of \fBK\fP, \fBM\fP or \fBG\fP which
correspond to kilobytes, megabytes and gigabytes, respectively. If no suffix is
provided the input is treated as bytes.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        args.regex_size_limit = Some(convert::human_readable_usize(&v)?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_regex_size_limit() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.regex_size_limit);

    #[cfg(target_pointer_width = "64")]
    {
        let args = parse_low_raw(["--regex-size-limit", "9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.regex_size_limit);

        let args = parse_low_raw(["--regex-size-limit=9G"]).unwrap();
        assert_eq!(Some(9 * (1 << 30)), args.regex_size_limit);

        let args =
            parse_low_raw(["--regex-size-limit=9G", "--regex-size-limit=0"])
                .unwrap();
        assert_eq!(Some(0), args.regex_size_limit);
    }

    let args = parse_low_raw(["--regex-size-limit=0K"]).unwrap();
    assert_eq!(Some(0), args.regex_size_limit);

    let args = parse_low_raw(["--regex-size-limit=0M"]).unwrap();
    assert_eq!(Some(0), args.regex_size_limit);

    let args = parse_low_raw(["--regex-size-limit=0G"]).unwrap();
    assert_eq!(Some(0), args.regex_size_limit);

    let result =
        parse_low_raw(["--regex-size-limit", "9999999999999999999999"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--regex-size-limit", "9999999999999999G"]);
    assert!(result.is_err(), "{result:?}");
}

/// -e/--regexp
#[derive(Debug)]
struct Regexp;

impl Flag for Regexp {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'e')
    }
    fn name_long(&self) -> &'static str {
        "regexp"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("PATTERN")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"A pattern to search for."
    }
    fn doc_long(&self) -> &'static str {
        r"
A pattern to search for. This option can be provided multiple times, where
all patterns given are searched, in addition to any patterns provided by
\flag{file}. Lines matching at least one of the provided patterns are printed.
This flag can also be used when searching for patterns that start with a dash.
.sp
For example, to search for the literal \fB\-foo\fP:
.sp
.EX
    rg \-e \-foo
.EE
.sp
You can also use the special \fB\-\-\fP delimiter to indicate that no more
flags will be provided. Namely, the following is equivalent to the above:
.sp
.EX
    rg \-\- \-foo
.EE
.sp
When \flag{file} or \flag{regexp} is used, then ripgrep treats all positional
arguments as files or directories to search.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let regexp = convert::string(v.unwrap_value())?;
        args.patterns.push(PatternSource::Regexp(regexp));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_regexp() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<PatternSource>::new(), args.patterns);

    let args = parse_low_raw(["--regexp", "foo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp=foo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["-e", "foo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["-efoo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp", "-foo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp=-foo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["-e", "-foo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["-e-foo"]).unwrap();
    assert_eq!(vec![PatternSource::Regexp("-foo".to_string())], args.patterns);

    let args = parse_low_raw(["--regexp=foo", "--regexp", "bar"]).unwrap();
    assert_eq!(
        vec![
            PatternSource::Regexp("foo".to_string()),
            PatternSource::Regexp("bar".to_string())
        ],
        args.patterns
    );

    // While we support invalid UTF-8 arguments in general, patterns must be
    // valid UTF-8.
    #[cfg(unix)]
    {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let bytes = &[b'A', 0xFF, b'Z'][..];
        let result = parse_low_raw([
            OsStr::from_bytes(b"-e"),
            OsStr::from_bytes(bytes),
        ]);
        assert!(result.is_err(), "{result:?}");
    }

    // Check that combining -e/--regexp and -f/--file works as expected.
    let args = parse_low_raw(["-efoo", "-fbar"]).unwrap();
    assert_eq!(
        vec![
            PatternSource::Regexp("foo".to_string()),
            PatternSource::File(PathBuf::from("bar"))
        ],
        args.patterns
    );

    let args = parse_low_raw(["-efoo", "-fbar", "-equux"]).unwrap();
    assert_eq!(
        vec![
            PatternSource::Regexp("foo".to_string()),
            PatternSource::File(PathBuf::from("bar")),
            PatternSource::Regexp("quux".to_string()),
        ],
        args.patterns
    );
}

/// -r/--replace
#[derive(Debug)]
struct Replace;

impl Flag for Replace {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'r')
    }
    fn name_long(&self) -> &'static str {
        "replace"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("REPLACEMENT")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Replace matches with the given text."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Replaces every match with the text given when printing results. Neither this
flag nor any other ripgrep flag will modify your files.
.sp
Capture group indices (e.g., \fB$\fP\fI5\fP) and names (e.g., \fB$\fP\fIfoo\fP)
are supported in the replacement string. Capture group indices are numbered
based on the position of the opening parenthesis of the group, where the
leftmost such group is \fB$\fP\fI1\fP. The special \fB$\fP\fI0\fP group
corresponds to the entire match.
.sp
The name of a group is formed by taking the longest string of letters, numbers
and underscores (i.e. \fB[_0-9A-Za-z]\fP) after the \fB$\fP. For example,
\fB$\fP\fI1a\fP will be replaced with the group named \fI1a\fP, not the
group at index \fI1\fP. If the group's name contains characters that aren't
letters, numbers or underscores, or you want to immediately follow the group
with another string, the name should be put inside braces. For example,
\fB${\fP\fI1\fP\fB}\fP\fIa\fP will take the content of the group at index
\fI1\fP and append \fIa\fP to the end of it.
.sp
If an index or name does not refer to a valid capture group, it will be
replaced with an empty string.
.sp
In shells such as Bash and zsh, you should wrap the pattern in single quotes
instead of double quotes. Otherwise, capture group indices will be replaced by
expanded shell variables which will most likely be empty.
.sp
To write a literal \fB$\fP, use \fB$$\fP.
.sp
Note that the replacement by default replaces each match, and not the entire
line. To replace the entire line, you should match the entire line.
.sp
This flag can be used with the \flag{only-matching} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.replace = Some(convert::string(v.unwrap_value())?.into());
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_replace() {
    use bstr::BString;

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.replace);

    let args = parse_low_raw(["--replace", "foo"]).unwrap();
    assert_eq!(Some(BString::from("foo")), args.replace);

    let args = parse_low_raw(["--replace", "-foo"]).unwrap();
    assert_eq!(Some(BString::from("-foo")), args.replace);

    let args = parse_low_raw(["-r", "foo"]).unwrap();
    assert_eq!(Some(BString::from("foo")), args.replace);

    let args = parse_low_raw(["-r", "foo", "-rbar"]).unwrap();
    assert_eq!(Some(BString::from("bar")), args.replace);

    let args = parse_low_raw(["-r", "foo", "-r", ""]).unwrap();
    assert_eq!(Some(BString::from("")), args.replace);
}

/// -z/--search-zip
#[derive(Debug)]
struct SearchZip;

impl Flag for SearchZip {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'z')
    }
    fn name_long(&self) -> &'static str {
        "search-zip"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-search-zip")
    }
    fn doc_category(&self) -> Category {
        Category::Input
    }
    fn doc_short(&self) -> &'static str {
        r"Search in compressed files."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to search in compressed files. Currently gzip,
bzip2, xz, LZ4, LZMA, Brotli and Zstd files are supported. This option expects
the decompression binaries (such as \fBgzip\fP) to be available in your
\fBPATH\fP. If the required binaries are not found, then ripgrep will not
emit an error messages by default. Use the \flag{debug} flag to see more
information.
.sp
Note that this flag does not make ripgrep search archive formats as directory
trees. It only makes ripgrep detect compressed files and then decompress them
before searching their contents as it would any other file.
.sp
This overrides the \flag{pre} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.search_zip = if v.unwrap_switch() {
            args.pre = None;
            true
        } else {
            false
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_search_zip() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.search_zip);

    let args = parse_low_raw(["--search-zip"]).unwrap();
    assert_eq!(true, args.search_zip);

    let args = parse_low_raw(["-z"]).unwrap();
    assert_eq!(true, args.search_zip);

    let args = parse_low_raw(["-z", "--no-search-zip"]).unwrap();
    assert_eq!(false, args.search_zip);

    let args = parse_low_raw(["--pre=foo", "--no-search-zip"]).unwrap();
    assert_eq!(Some(PathBuf::from("foo")), args.pre);
    assert_eq!(false, args.search_zip);

    let args = parse_low_raw(["--pre=foo", "--search-zip"]).unwrap();
    assert_eq!(None, args.pre);
    assert_eq!(true, args.search_zip);

    let args = parse_low_raw(["--pre=foo", "-z", "--no-search-zip"]).unwrap();
    assert_eq!(None, args.pre);
    assert_eq!(false, args.search_zip);
}

/// -S/--smart-case
#[derive(Debug)]
struct SmartCase;

impl Flag for SmartCase {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'S')
    }
    fn name_long(&self) -> &'static str {
        "smart-case"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Smart case search."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to searches case insensitively if the pattern is
all lowercase. Otherwise, ripgrep will search case sensitively.
.sp
A pattern is considered all lowercase if both of the following rules hold:
.sp
.IP \(bu 3n
First, the pattern contains at least one literal character. For example,
\fBa\\w\fP contains a literal (\fBa\fP) but just \fB\\w\fP does not.
.sp
.IP \(bu 3n
Second, of the literals in the pattern, none of them are considered to be
uppercase according to Unicode. For example, \fBfoo\\pL\fP has no uppercase
literals but \fBFoo\\pL\fP does.
.PP
This overrides the \flag{case-sensitive} and \flag{ignore-case} flags.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--smart-case flag has no negation");
        args.case = CaseMode::Smart;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_smart_case() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["--smart-case"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-S"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-S", "-s"]).unwrap();
    assert_eq!(CaseMode::Sensitive, args.case);

    let args = parse_low_raw(["-S", "-i"]).unwrap();
    assert_eq!(CaseMode::Insensitive, args.case);

    let args = parse_low_raw(["-s", "-S"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);

    let args = parse_low_raw(["-i", "-S"]).unwrap();
    assert_eq!(CaseMode::Smart, args.case);
}

/// --sort-files
#[derive(Debug)]
struct SortFiles;

impl Flag for SortFiles {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "sort-files"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-sort-files")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"(DEPRECATED) Sort results by file path."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \fB\-\-sort=path\fP instead.
.sp
This flag instructs ripgrep to sort search results by file path
lexicographically in ascending order. Note that this currently disables all
parallelism and runs search in a single thread.
.sp
This flag overrides \flag{sort} and \flag{sortr}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.sort = if v.unwrap_switch() {
            Some(SortMode { reverse: false, kind: SortModeKind::Path })
        } else {
            None
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_sort_files() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort-files"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort-files", "--no-sort-files"]).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort", "created", "--sort-files"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort-files", "--sort", "created"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sortr", "created", "--sort-files"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort-files", "--sortr", "created"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sort=path", "--no-sort-files"]).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sortr=path", "--no-sort-files"]).unwrap();
    assert_eq!(None, args.sort);
}

/// --sort
#[derive(Debug)]
struct Sort;

impl Flag for Sort {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "sort"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SORTBY")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Sort results in ascending order."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag enables sorting of results in ascending order. The possible values
for this flag are:
.sp
.TP 12
\fBnone\fP
(Default) Do not sort results. Fastest. Can be multi-threaded.
.TP 12
\fBpath\fP
Sort by file path. Always single-threaded. The order is determined by sorting
files in each directory entry during traversal. This means that given the files
\fBa/b\fP and \fBa+\fP, the latter will sort after the former even though
\fB+\fP would normally sort before \fB/\fP.
.TP 12
\fBmodified\fP
Sort by the last modified time on a file. Always single-threaded.
.TP 12
\fBaccessed\fP
Sort by the last accessed time on a file. Always single-threaded.
.TP 12
\fBcreated\fP
Sort by the creation time on a file. Always single-threaded.
.PP
If the chosen (manually or by-default) sorting criteria isn't available on your
system (for example, creation time is not available on ext4 file systems), then
ripgrep will attempt to detect this, print an error and exit without searching.
.sp
To sort results in reverse or descending order, use the \flag{sortr} flag.
Also, this flag overrides \flag{sortr}.
.sp
Note that sorting results currently always forces ripgrep to abandon
parallelism and run in a single thread.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["none", "path", "modified", "accessed", "created"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let kind = match convert::str(&v.unwrap_value())? {
            "none" => {
                args.sort = None;
                return Ok(());
            }
            "path" => SortModeKind::Path,
            "modified" => SortModeKind::LastModified,
            "accessed" => SortModeKind::LastAccessed,
            "created" => SortModeKind::Created,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        args.sort = Some(SortMode { reverse: false, kind });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_sort() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort", "path"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sort", "path", "--sort=created"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sort=none"]).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort", "path", "--sort=none"]).unwrap();
    assert_eq!(None, args.sort);
}

/// --sortr
#[derive(Debug)]
struct Sortr;

impl Flag for Sortr {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "sortr"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("SORTBY")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Sort results in descending order."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag enables sorting of results in descending order. The possible values
for this flag are:
.sp
.TP 12
\fBnone\fP
(Default) Do not sort results. Fastest. Can be multi-threaded.
.TP 12
\fBpath\fP
Sort by file path. Always single-threaded. The order is determined by sorting
files in each directory entry during traversal. This means that given the files
\fBa/b\fP and \fBa+\fP, the latter will sort before the former even though
\fB+\fP would normally sort after \fB/\fP when doing a reverse lexicographic
sort.
.TP 12
\fBmodified\fP
Sort by the last modified time on a file. Always single-threaded.
.TP 12
\fBaccessed\fP
Sort by the last accessed time on a file. Always single-threaded.
.TP 12
\fBcreated\fP
Sort by the creation time on a file. Always single-threaded.
.PP
If the chosen (manually or by-default) sorting criteria isn't available on your
system (for example, creation time is not available on ext4 file systems), then
ripgrep will attempt to detect this, print an error and exit without searching.
.sp
To sort results in ascending order, use the \flag{sort} flag. Also, this flag
overrides \flag{sort}.
.sp
Note that sorting results currently always forces ripgrep to abandon
parallelism and run in a single thread.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["none", "path", "modified", "accessed", "created"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let kind = match convert::str(&v.unwrap_value())? {
            "none" => {
                args.sort = None;
                return Ok(());
            }
            "path" => SortModeKind::Path,
            "modified" => SortModeKind::LastModified,
            "accessed" => SortModeKind::LastAccessed,
            "created" => SortModeKind::Created,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        args.sort = Some(SortMode { reverse: true, kind });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_sortr() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sortr", "path"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sortr", "path", "--sortr=created"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Created }),
        args.sort
    );

    let args = parse_low_raw(["--sortr=none"]).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sortr", "path", "--sortr=none"]).unwrap();
    assert_eq!(None, args.sort);

    let args = parse_low_raw(["--sort=path", "--sortr=path"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: true, kind: SortModeKind::Path }),
        args.sort
    );

    let args = parse_low_raw(["--sortr=path", "--sort=path"]).unwrap();
    assert_eq!(
        Some(SortMode { reverse: false, kind: SortModeKind::Path }),
        args.sort
    );
}

/// --stats
#[derive(Debug)]
struct Stats;

impl Flag for Stats {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "stats"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-stats")
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Print statistics about the search."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will print aggregate statistics about the search. When
this flag is present, ripgrep will print at least the following stats to
stdout at the end of the search: number of matched lines, number of files with
matches, number of files searched, and the time taken for the entire search to
complete.
.sp
This set of aggregate statistics may expand over time.
.sp
This flag is always and implicitly enabled when \flag{json} is used.
.sp
Note that this flag has no effect if \flag{files}, \flag{files-with-matches} or
\flag{files-without-match} is passed.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.stats = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_stats() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.stats);

    let args = parse_low_raw(["--stats"]).unwrap();
    assert_eq!(true, args.stats);

    let args = parse_low_raw(["--stats", "--no-stats"]).unwrap();
    assert_eq!(false, args.stats);
}

/// --stop-on-nonmatch
#[derive(Debug)]
struct StopOnNonmatch;

impl Flag for StopOnNonmatch {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "stop-on-nonmatch"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Stop searching after a non-match."
    }
    fn doc_long(&self) -> &'static str {
        r"
Enabling this option will cause ripgrep to stop reading a file once it
encounters a non-matching line after it has encountered a matching line.
This is useful if it is expected that all matches in a given file will be on
sequential lines, for example due to the lines being sorted.
.sp
This overrides the \flag{multiline} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--stop-on-nonmatch has no negation");
        args.stop_on_nonmatch = true;
        args.multiline = false;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_stop_on_nonmatch() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.stop_on_nonmatch);

    let args = parse_low_raw(["--stop-on-nonmatch"]).unwrap();
    assert_eq!(true, args.stop_on_nonmatch);

    let args = parse_low_raw(["--stop-on-nonmatch", "-U"]).unwrap();
    assert_eq!(true, args.multiline);
    assert_eq!(false, args.stop_on_nonmatch);

    let args = parse_low_raw(["-U", "--stop-on-nonmatch"]).unwrap();
    assert_eq!(false, args.multiline);
    assert_eq!(true, args.stop_on_nonmatch);

    let args =
        parse_low_raw(["--stop-on-nonmatch", "--no-multiline"]).unwrap();
    assert_eq!(false, args.multiline);
    assert_eq!(true, args.stop_on_nonmatch);
}

/// -a/--text
#[derive(Debug)]
struct Text;

impl Flag for Text {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'a')
    }
    fn name_long(&self) -> &'static str {
        "text"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-text")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Search binary files as if they were text."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to search binary files as if they were text. When
this flag is present, ripgrep's binary file detection is disabled. This means
that when a binary file is searched, its contents may be printed if there is
a match. This may cause escape codes to be printed that alter the behavior of
your terminal.
.sp
When binary file detection is enabled, it is imperfect. In general, it uses
a simple heuristic. If a \fBNUL\fP byte is seen during search, then the file
is considered binary and searching stops (unless this flag is present).
Alternatively, if the \flag{binary} flag is used, then ripgrep will only quit
when it sees a \fBNUL\fP byte after it sees a match (or searches the entire
file).
.sp
This flag overrides the \flag{binary} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.binary = if v.unwrap_switch() {
            BinaryMode::AsText
        } else {
            BinaryMode::Auto
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_text() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--text"]).unwrap();
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a"]).unwrap();
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--no-text"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["-a", "--binary"]).unwrap();
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let args = parse_low_raw(["--binary", "-a"]).unwrap();
    assert_eq!(BinaryMode::AsText, args.binary);

    let args = parse_low_raw(["-a", "--no-binary"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--binary", "--no-text"]).unwrap();
    assert_eq!(BinaryMode::Auto, args.binary);
}

/// -j/--threads
#[derive(Debug)]
struct Threads;

impl Flag for Threads {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'j')
    }
    fn name_long(&self) -> &'static str {
        "threads"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("NUM")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Set the approximate number of threads to use."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag sets the approximate number of threads to use. A value of \fB0\fP
(which is the default) causes ripgrep to choose the thread count using
heuristics.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let threads = convert::usize(&v.unwrap_value())?;
        args.threads = if threads == 0 { None } else { Some(threads) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_threads() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.threads);

    let args = parse_low_raw(["--threads", "5"]).unwrap();
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j", "5"]).unwrap();
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j5"]).unwrap();
    assert_eq!(Some(5), args.threads);

    let args = parse_low_raw(["-j5", "-j10"]).unwrap();
    assert_eq!(Some(10), args.threads);

    let args = parse_low_raw(["-j5", "-j0"]).unwrap();
    assert_eq!(None, args.threads);
}

/// --trace
#[derive(Debug)]
struct Trace;

impl Flag for Trace {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "trace"
    }
    fn doc_category(&self) -> Category {
        Category::Logging
    }
    fn doc_short(&self) -> &'static str {
        r"Show trace messages."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show trace messages. This shows even more detail than the \flag{debug}
flag. Generally, one should only use this if \flag{debug} doesn't emit the
information you're looking for.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--trace can only be enabled");
        args.logging = Some(LoggingMode::Trace);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_trace() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.logging);

    let args = parse_low_raw(["--trace"]).unwrap();
    assert_eq!(Some(LoggingMode::Trace), args.logging);

    let args = parse_low_raw(["--debug", "--trace"]).unwrap();
    assert_eq!(Some(LoggingMode::Trace), args.logging);
}

/// --trim
#[derive(Debug)]
struct Trim;

impl Flag for Trim {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "trim"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-trim")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Trim prefix whitespace from matches."
    }
    fn doc_long(&self) -> &'static str {
        r"
When set, all ASCII whitespace at the beginning of each line printed will be
removed.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.trim = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_trim() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.trim);

    let args = parse_low_raw(["--trim"]).unwrap();
    assert_eq!(true, args.trim);

    let args = parse_low_raw(["--trim", "--no-trim"]).unwrap();
    assert_eq!(false, args.trim);
}

/// -t/--type
#[derive(Debug)]
struct Type;

impl Flag for Type {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b't')
    }
    fn name_long(&self) -> &'static str {
        "type"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("TYPE")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Only search files matching TYPE."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag limits ripgrep to searching files matching \fITYPE\fP. Multiple
\flag{type} flags may be provided.
.sp
This flag supports the special value \fBall\fP, which will behave as if
\flag{type} was provided for every file type supported by ripgrep (including
any custom file types). The end result is that \fB\-\-type=all\fP causes
ripgrep to search in "whitelist" mode, where it will only search files it
recognizes via its type definitions.
.sp
Note that this flag has lower precedence than both the \flag{glob} flag and
any rules found in ignore files.
.sp
To see the list of available file types, use the \flag{type-list} flag.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Filetype
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.type_changes.push(TypeChange::Select {
            name: convert::string(v.unwrap_value())?,
        });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_type() {
    let select = |name: &str| TypeChange::Select { name: name.to_string() };

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<TypeChange>::new(), args.type_changes);

    let args = parse_low_raw(["--type", "rust"]).unwrap();
    assert_eq!(vec![select("rust")], args.type_changes);

    let args = parse_low_raw(["-t", "rust"]).unwrap();
    assert_eq!(vec![select("rust")], args.type_changes);

    let args = parse_low_raw(["-trust"]).unwrap();
    assert_eq!(vec![select("rust")], args.type_changes);

    let args = parse_low_raw(["-trust", "-tpython"]).unwrap();
    assert_eq!(vec![select("rust"), select("python")], args.type_changes);

    let args = parse_low_raw(["-tabcdefxyz"]).unwrap();
    assert_eq!(vec![select("abcdefxyz")], args.type_changes);
}

/// --type-add
#[derive(Debug)]
struct TypeAdd;

impl Flag for TypeAdd {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "type-add"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("TYPESPEC")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Add a new glob for a file type."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag adds a new glob for a particular file type. Only one glob can be
added at a time. Multiple \flag{type-add} flags can be provided. Unless
\flag{type-clear} is used, globs are added to any existing globs defined inside
of ripgrep.
.sp
Note that this must be passed to every invocation of ripgrep. Type settings are
not persisted. See \fBCONFIGURATION FILES\fP for a workaround.
.sp
Example:
.sp
.EX
    rg \-\-type\-add 'foo:*.foo' -tfoo \fIPATTERN\fP
.EE
.sp
This flag can also be used to include rules from other types with the special
include directive. The include directive permits specifying one or more other
type names (separated by a comma) that have been defined and its rules will
automatically be imported into the type specified. For example, to create a
type called src that matches C++, Python and Markdown files, one can use:
.sp
.EX
    \-\-type\-add 'src:include:cpp,py,md'
.EE
.sp
Additional glob rules can still be added to the src type by using this flag
again:
.sp
.EX
    \-\-type\-add 'src:include:cpp,py,md' \-\-type\-add 'src:*.foo'
.EE
.sp
Note that type names must consist only of Unicode letters or numbers.
Punctuation characters are not allowed.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.type_changes
            .push(TypeChange::Add { def: convert::string(v.unwrap_value())? });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_type_add() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<TypeChange>::new(), args.type_changes);

    let args = parse_low_raw(["--type-add", "foo"]).unwrap();
    assert_eq!(
        vec![TypeChange::Add { def: "foo".to_string() }],
        args.type_changes
    );

    let args = parse_low_raw(["--type-add", "foo", "--type-add=bar"]).unwrap();
    assert_eq!(
        vec![
            TypeChange::Add { def: "foo".to_string() },
            TypeChange::Add { def: "bar".to_string() }
        ],
        args.type_changes
    );
}

/// --type-clear
#[derive(Debug)]
struct TypeClear;

impl Flag for TypeClear {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "type-clear"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("TYPE")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Clear globs for a file type."
    }
    fn doc_long(&self) -> &'static str {
        r"
Clear the file type globs previously defined for \fITYPE\fP. This clears any
previously defined globs for the \fITYPE\fP, but globs can be added after this
flag.
.sp
Note that this must be passed to every invocation of ripgrep. Type settings are
not persisted. See \fBCONFIGURATION FILES\fP for a workaround.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.type_changes.push(TypeChange::Clear {
            name: convert::string(v.unwrap_value())?,
        });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_type_clear() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<TypeChange>::new(), args.type_changes);

    let args = parse_low_raw(["--type-clear", "foo"]).unwrap();
    assert_eq!(
        vec![TypeChange::Clear { name: "foo".to_string() }],
        args.type_changes
    );

    let args =
        parse_low_raw(["--type-clear", "foo", "--type-clear=bar"]).unwrap();
    assert_eq!(
        vec![
            TypeChange::Clear { name: "foo".to_string() },
            TypeChange::Clear { name: "bar".to_string() }
        ],
        args.type_changes
    );
}

/// --type-not
#[derive(Debug)]
struct TypeNot;

impl Flag for TypeNot {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'T')
    }
    fn name_long(&self) -> &'static str {
        "type-not"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("TYPE")
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r"Do not search files matching TYPE."
    }
    fn doc_long(&self) -> &'static str {
        r#"
Do not search files matching \fITYPE\fP. Multiple \flag{type-not} flags may be
provided. Use the \flag{type-list} flag to list all available types.
.sp
This flag supports the special value \fBall\fP, which will behave
as if \flag{type-not} was provided for every file type supported by
ripgrep (including any custom file types). The end result is that
\fB\-\-type\-not=all\fP causes ripgrep to search in "blacklist" mode, where it
will only search files that are unrecognized by its type definitions.
.sp
To see the list of available file types, use the \flag{type-list} flag.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Filetype
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.type_changes.push(TypeChange::Negate {
            name: convert::string(v.unwrap_value())?,
        });
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_type_not() {
    let select = |name: &str| TypeChange::Select { name: name.to_string() };
    let negate = |name: &str| TypeChange::Negate { name: name.to_string() };

    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Vec::<TypeChange>::new(), args.type_changes);

    let args = parse_low_raw(["--type-not", "rust"]).unwrap();
    assert_eq!(vec![negate("rust")], args.type_changes);

    let args = parse_low_raw(["-T", "rust"]).unwrap();
    assert_eq!(vec![negate("rust")], args.type_changes);

    let args = parse_low_raw(["-Trust"]).unwrap();
    assert_eq!(vec![negate("rust")], args.type_changes);

    let args = parse_low_raw(["-Trust", "-Tpython"]).unwrap();
    assert_eq!(vec![negate("rust"), negate("python")], args.type_changes);

    let args = parse_low_raw(["-Tabcdefxyz"]).unwrap();
    assert_eq!(vec![negate("abcdefxyz")], args.type_changes);

    let args = parse_low_raw(["-Trust", "-ttoml", "-Tjson"]).unwrap();
    assert_eq!(
        vec![negate("rust"), select("toml"), negate("json")],
        args.type_changes
    );
}

/// --type-list
#[derive(Debug)]
struct TypeList;

impl Flag for TypeList {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "type-list"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Show all supported file types."
    }
    fn doc_long(&self) -> &'static str {
        r"
Show all supported file types and their corresponding globs. This takes any
\flag{type-add} and \flag{type-clear} flags given into account. Each type is
printed on its own line, followed by a \fB:\fP and then a comma-delimited list
of globs for that type on the same line.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--type-list has no negation");
        args.mode.update(Mode::Types);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_type_list() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(Mode::Search(SearchMode::Standard), args.mode);

    let args = parse_low_raw(["--type-list"]).unwrap();
    assert_eq!(Mode::Types, args.mode);
}

/// -u/--unrestricted
#[derive(Debug)]
struct Unrestricted;

impl Flag for Unrestricted {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'u')
    }
    fn name_long(&self) -> &'static str {
        "unrestricted"
    }
    fn doc_category(&self) -> Category {
        Category::Filter
    }
    fn doc_short(&self) -> &'static str {
        r#"Reduce the level of "smart" filtering."#
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag reduces the level of "smart" filtering. Repeated uses (up to 3) reduces
the filtering even more. When repeated three times, ripgrep will search every
file in a directory tree.
.sp
A single \flag{unrestricted} flag is equivalent to \flag{no-ignore}. Two
\flag{unrestricted} flags is equivalent to \flag{no-ignore} \flag{hidden}.
Three \flag{unrestricted} flags is equivalent to \flag{no-ignore} \flag{hidden}
\flag{binary}.
.sp
The only filtering ripgrep still does when \fB-uuu\fP is given is to skip
symbolic links and to avoid printing matches from binary files. Symbolic links
can be followed via the \flag{follow} flag, and binary files can be treated as
text files via the \flag{text} flag.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--unrestricted has no negation");
        args.unrestricted = args.unrestricted.saturating_add(1);
        anyhow::ensure!(
            args.unrestricted <= 3,
            "flag can only be repeated up to 3 times"
        );
        if args.unrestricted == 1 {
            NoIgnore.update(FlagValue::Switch(true), args)?;
        } else if args.unrestricted == 2 {
            Hidden.update(FlagValue::Switch(true), args)?;
        } else {
            assert_eq!(args.unrestricted, 3);
            Binary.update(FlagValue::Switch(true), args)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_unrestricted() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.no_ignore_vcs);
    assert_eq!(false, args.hidden);
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--unrestricted"]).unwrap();
    assert_eq!(true, args.no_ignore_vcs);
    assert_eq!(false, args.hidden);
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["--unrestricted", "-u"]).unwrap();
    assert_eq!(true, args.no_ignore_vcs);
    assert_eq!(true, args.hidden);
    assert_eq!(BinaryMode::Auto, args.binary);

    let args = parse_low_raw(["-uuu"]).unwrap();
    assert_eq!(true, args.no_ignore_vcs);
    assert_eq!(true, args.hidden);
    assert_eq!(BinaryMode::SearchAndSuppress, args.binary);

    let result = parse_low_raw(["-uuuu"]);
    assert!(result.is_err(), "{result:?}");
}

/// --version
#[derive(Debug)]
struct Version;

impl Flag for Version {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'V')
    }
    fn name_long(&self) -> &'static str {
        "version"
    }
    fn doc_category(&self) -> Category {
        Category::OtherBehaviors
    }
    fn doc_short(&self) -> &'static str {
        r"Print ripgrep's version."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints ripgrep's version. This also may print other relevant
information, such as the presence of target specific optimizations and the
\fBgit\fP revision that this build of ripgrep was compiled from.
"
    }

    fn update(&self, v: FlagValue, _: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--version has no negation");
        // Since this flag has different semantics for -V and --version and the
        // Flag trait doesn't support encoding this sort of thing, we handle it
        // as a special case in the parser.
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_version() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.special);

    let args = parse_low_raw(["-V"]).unwrap();
    assert_eq!(Some(SpecialMode::VersionShort), args.special);

    let args = parse_low_raw(["--version"]).unwrap();
    assert_eq!(Some(SpecialMode::VersionLong), args.special);

    let args = parse_low_raw(["-V", "--version"]).unwrap();
    assert_eq!(Some(SpecialMode::VersionLong), args.special);

    let args = parse_low_raw(["--version", "-V"]).unwrap();
    assert_eq!(Some(SpecialMode::VersionShort), args.special);
}

/// --vimgrep
#[derive(Debug)]
struct Vimgrep;

impl Flag for Vimgrep {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "vimgrep"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print results in a vim compatible format."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to print results with every match on its own line,
including line numbers and column numbers.
.sp
With this option, a line with more than one match will be printed in its
entirety more than once. For that reason, the total amount of output as a
result of this flag can be quadratic in the size of the input. For example,
if the pattern matches every byte in an input file, then each line will be
repeated for every byte matched. For this reason, users should only use this
flag when there is no other choice. Editor integrations should prefer some
other way of reading results from ripgrep, such as via the \flag{json} flag.
One alternative to avoiding exorbitant memory usage is to force ripgrep into
single threaded mode with the \flag{threads} flag. Note though that this will
not impact the total size of the output, just the heap memory that ripgrep will
use.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &[]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--vimgrep has no negation");
        args.vimgrep = true;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_vimgrep() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(false, args.vimgrep);

    let args = parse_low_raw(["--vimgrep"]).unwrap();
    assert_eq!(true, args.vimgrep);
}

/// --with-filename
#[derive(Debug)]
struct WithFilename;

impl Flag for WithFilename {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'H')
    }
    fn name_long(&self) -> &'static str {
        "with-filename"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Print the file path with each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to print the file path for each matching line.
This is the default when more than one file is searched. If \flag{heading} is
enabled (the default when printing to a tty), the file path will be shown above
clusters of matches from each file; otherwise, the file name will be shown as a
prefix for each matched line.
.sp
This flag overrides \flag{no-filename}.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--with-filename has no defined negation");
        args.with_filename = Some(true);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_with_filename() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.with_filename);

    let args = parse_low_raw(["--with-filename"]).unwrap();
    assert_eq!(Some(true), args.with_filename);

    let args = parse_low_raw(["-H"]).unwrap();
    assert_eq!(Some(true), args.with_filename);
}

/// --no-filename
#[derive(Debug)]
struct WithFilenameNo;

impl Flag for WithFilenameNo {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'I')
    }
    fn name_long(&self) -> &'static str {
        "no-filename"
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Never print the path with each matching line."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag instructs ripgrep to never print the file path with each matching
line. This is the default when ripgrep is explicitly instructed to search one
file or stdin.
.sp
This flag overrides \flag{with-filename}.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &[]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--no-filename has no defined negation");
        args.with_filename = Some(false);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_with_filename_no() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.with_filename);

    let args = parse_low_raw(["--no-filename"]).unwrap();
    assert_eq!(Some(false), args.with_filename);

    let args = parse_low_raw(["-I"]).unwrap();
    assert_eq!(Some(false), args.with_filename);

    let args = parse_low_raw(["-I", "-H"]).unwrap();
    assert_eq!(Some(true), args.with_filename);

    let args = parse_low_raw(["-H", "-I"]).unwrap();
    assert_eq!(Some(false), args.with_filename);
}

/// -w/--word-regexp
#[derive(Debug)]
struct WordRegexp;

impl Flag for WordRegexp {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'w')
    }
    fn name_long(&self) -> &'static str {
        "word-regexp"
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Show matches surrounded by word boundaries."
    }
    fn doc_long(&self) -> &'static str {
        r"
When enabled, ripgrep will only show matches surrounded by word boundaries.
This is equivalent to surrounding every pattern with \fB\\b{start-half}\fP
and \fB\\b{end-half}\fP.
.sp
This overrides the \flag{line-regexp} flag.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--word-regexp has no negation");
        args.boundary = Some(BoundaryMode::Word);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_word_regexp() {
    let args = parse_low_raw(None::<&str>).unwrap();
    assert_eq!(None, args.boundary);

    let args = parse_low_raw(["--word-regexp"]).unwrap();
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-w"]).unwrap();
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-x", "-w"]).unwrap();
    assert_eq!(Some(BoundaryMode::Word), args.boundary);

    let args = parse_low_raw(["-w", "-x"]).unwrap();
    assert_eq!(Some(BoundaryMode::Line), args.boundary);
}

mod convert {
    use std::ffi::{OsStr, OsString};

    use anyhow::Context;

    pub(super) fn str(v: &OsStr) -> anyhow::Result<&str> {
        let Some(s) = v.to_str() else {
            anyhow::bail!("value is not valid UTF-8")
        };
        Ok(s)
    }

    pub(super) fn string(v: OsString) -> anyhow::Result<String> {
        let Ok(s) = v.into_string() else {
            anyhow::bail!("value is not valid UTF-8")
        };
        Ok(s)
    }

    pub(super) fn usize(v: &OsStr) -> anyhow::Result<usize> {
        str(v)?.parse().context("value is not a valid number")
    }

    pub(super) fn u64(v: &OsStr) -> anyhow::Result<u64> {
        str(v)?.parse().context("value is not a valid number")
    }

    pub(super) fn human_readable_u64(v: &OsStr) -> anyhow::Result<u64> {
        grep::cli::parse_human_readable_size(str(v)?).context("invalid size")
    }

    pub(super) fn human_readable_usize(v: &OsStr) -> anyhow::Result<usize> {
        let size = human_readable_u64(v)?;
        let Ok(size) = usize::try_from(size) else {
            anyhow::bail!("size is too big")
        };
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_shorts() {
        let mut total = vec![false; 128];
        for byte in 0..=0x7F {
            match byte {
                b'.' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => {
                    total[usize::from(byte)] = true
                }
                _ => continue,
            }
        }

        let mut taken = vec![false; 128];
        for flag in FLAGS.iter() {
            let Some(short) = flag.name_short() else { continue };
            taken[usize::from(short)] = true;
        }

        for byte in 0..=0x7F {
            if total[usize::from(byte)] && !taken[usize::from(byte)] {
                eprintln!("{}", char::from(byte));
            }
        }
    }

    #[test]
    fn shorts_all_ascii_alphanumeric() {
        for flag in FLAGS.iter() {
            let Some(byte) = flag.name_short() else { continue };
            let long = flag.name_long();
            assert!(
                byte.is_ascii_alphanumeric() || byte == b'.',
                "\\x{byte:0X} is not a valid short flag for {long}",
            )
        }
    }

    #[test]
    fn longs_all_ascii_alphanumeric() {
        for flag in FLAGS.iter() {
            let long = flag.name_long();
            let count = long.chars().count();
            assert!(count >= 2, "flag '{long}' is less than 2 characters");
            assert!(
                long.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "flag '{long}' does not match ^[-0-9A-Za-z]+$",
            );
            for alias in flag.aliases() {
                let count = alias.chars().count();
                assert!(
                    count >= 2,
                    "flag '{long}' has alias '{alias}' that is \
                     less than 2 characters",
                );
                assert!(
                    alias
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                    "flag '{long}' has alias '{alias}' that does not \
                     match ^[-0-9A-Za-z]+$",
                );
            }
            let Some(negated) = flag.name_negated() else { continue };
            let count = negated.chars().count();
            assert!(
                count >= 2,
                "flag '{long}' has negation '{negated}' that is \
                 less than 2 characters",
            );
            assert!(
                negated.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "flag '{long}' has negation '{negated}' that \
                 does not match ^[-0-9A-Za-z]+$",
            );
        }
    }

    #[test]
    fn shorts_no_duplicates() {
        let mut taken = vec![false; 128];
        for flag in FLAGS.iter() {
            let Some(short) = flag.name_short() else { continue };
            let long = flag.name_long();
            assert!(
                !taken[usize::from(short)],
                "flag {long} has duplicate short flag {}",
                char::from(short)
            );
            taken[usize::from(short)] = true;
        }
    }

    #[test]
    fn longs_no_duplicates() {
        use std::collections::BTreeSet;

        let mut taken = BTreeSet::new();
        for flag in FLAGS.iter() {
            let long = flag.name_long();
            assert!(taken.insert(long), "flag {long} has a duplicate name");
            for alias in flag.aliases() {
                assert!(
                    taken.insert(alias),
                    "flag {long} has an alias {alias} that is duplicative"
                );
            }
            let Some(negated) = flag.name_negated() else { continue };
            assert!(
                taken.insert(negated),
                "negated flag {negated} has a duplicate name"
            );
        }
    }

    #[test]
    fn non_switches_have_variable_names() {
        for flag in FLAGS.iter() {
            if flag.is_switch() {
                continue;
            }
            let long = flag.name_long();
            assert!(
                flag.doc_variable().is_some(),
                "flag '{long}' should have a variable name"
            );
        }
    }

    #[test]
    fn switches_have_no_choices() {
        for flag in FLAGS.iter() {
            if !flag.is_switch() {
                continue;
            }
            let long = flag.name_long();
            let choices = flag.doc_choices();
            assert!(
                choices.is_empty(),
                "switch flag '{long}' \
                 should not have any choices but has some: {choices:?}",
            );
        }
    }

    #[test]
    fn choices_ascii_alphanumeric() {
        for flag in FLAGS.iter() {
            let long = flag.name_long();
            for choice in flag.doc_choices() {
                assert!(
                    choice.chars().all(|c| c.is_ascii_alphanumeric()
                        || c == '-'
                        || c == ':'),
                    "choice '{choice}' for flag '{long}' does not match \
                     ^[-:0-9A-Za-z]+$",
                )
            }
        }
    }
}
