use std::collections::HashMap;

use grep_matcher::{
    ByteSet, Captures, LineMatchKind, LineTerminator, Match, Matcher, NoError,
};
use regex::bytes::{CaptureLocations, Regex};

use crate::config::{Config, ConfiguredHIR};
use crate::crlf::CRLFMatcher;
use crate::error::Error;
use crate::multi::MultiLiteralMatcher;
use crate::word::WordMatcher;

/// A builder for constructing a `Matcher` using regular expressions.
///
/// This builder re-exports many of the same options found on the regex crate's
/// builder, in addition to a few other options such as smart case, word
/// matching and the ability to set a line terminator which may enable certain
/// types of optimizations.
///
/// The syntax supported is documented as part of the regex crate:
/// <https://docs.rs/regex/#syntax>.
#[derive(Clone, Debug)]
pub struct RegexMatcherBuilder {
    config: Config,
}

impl Default for RegexMatcherBuilder {
    fn default() -> RegexMatcherBuilder {
        RegexMatcherBuilder::new()
    }
}

impl RegexMatcherBuilder {
    /// Create a new builder for configuring a regex matcher.
    pub fn new() -> RegexMatcherBuilder {
        RegexMatcherBuilder { config: Config::default() }
    }

    /// Build a new matcher using the current configuration for the provided
    /// pattern.
    ///
    /// The syntax supported is documented as part of the regex crate:
    /// <https://docs.rs/regex/#syntax>.
    pub fn build(&self, pattern: &str) -> Result<RegexMatcher, Error> {
        let chir = self.config.hir(pattern)?;
        let fast_line_regex = chir.fast_line_regex()?;
        let non_matching_bytes = chir.non_matching_bytes();
        if let Some(ref re) = fast_line_regex {
            log::debug!("extracted fast line regex: {:?}", re);
        }

        let matcher = RegexMatcherImpl::new(&chir)?;
        log::trace!("final regex: {:?}", matcher.regex());
        Ok(RegexMatcher {
            config: self.config.clone(),
            matcher,
            fast_line_regex,
            non_matching_bytes,
        })
    }

    /// Build a new matcher from a plain alternation of literals.
    ///
    /// Depending on the configuration set by the builder, this may be able to
    /// build a matcher substantially faster than by joining the patterns with
    /// a `|` and calling `build`.
    pub fn build_literals<B: AsRef<str>>(
        &self,
        literals: &[B],
    ) -> Result<RegexMatcher, Error> {
        let mut has_escape = false;
        let mut slices = vec![];
        for lit in literals {
            slices.push(lit.as_ref());
            has_escape = has_escape || lit.as_ref().contains('\\');
        }
        // Even when we have a fixed set of literals, we might still want to
        // use the regex engine. Specifically, if any string has an escape
        // in it, then we probably can't feed it to Aho-Corasick without
        // removing the escape. Additionally, if there are any particular
        // special match semantics we need to honor, that Aho-Corasick isn't
        // enough. Finally, the regex engine can do really well with a small
        // number of literals (at time of writing, this is changing soon), so
        // we use it when there's a small set.
        //
        // Yes, this is one giant hack. Ideally, this entirely separate literal
        // matcher that uses Aho-Corasick would be pushed down into the regex
        // engine.
        if has_escape
            || !self.config.can_plain_aho_corasick()
            || literals.len() < 40
        {
            return self.build(&slices.join("|"));
        }

        let matcher = MultiLiteralMatcher::new(&slices)?;
        let imp = RegexMatcherImpl::MultiLiteral(matcher);
        Ok(RegexMatcher {
            config: self.config.clone(),
            matcher: imp,
            fast_line_regex: None,
            non_matching_bytes: ByteSet::empty(),
        })
    }

    /// Set the value for the case insensitive (`i`) flag.
    ///
    /// When enabled, letters in the pattern will match both upper case and
    /// lower case variants.
    pub fn case_insensitive(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.case_insensitive = yes;
        self
    }

    /// Whether to enable "smart case" or not.
    ///
    /// When smart case is enabled, the builder will automatically enable
    /// case insensitive matching based on how the pattern is written. Namely,
    /// case insensitive mode is enabled when both of the following things
    /// are true:
    ///
    /// 1. The pattern contains at least one literal character. For example,
    ///    `a\w` contains a literal (`a`) but `\w` does not.
    /// 2. Of the literals in the pattern, none of them are considered to be
    ///    uppercase according to Unicode. For example, `foo\pL` has no
    ///    uppercase literals but `Foo\pL` does.
    pub fn case_smart(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.case_smart = yes;
        self
    }

    /// Set the value for the multi-line matching (`m`) flag.
    ///
    /// When enabled, `^` matches the beginning of lines and `$` matches the
    /// end of lines.
    ///
    /// By default, they match beginning/end of the input.
    pub fn multi_line(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.multi_line = yes;
        self
    }

    /// Set the value for the any character (`s`) flag, where in `.` matches
    /// anything when `s` is set and matches anything except for new line when
    /// it is not set (the default).
    ///
    /// N.B. "matches anything" means "any byte" when Unicode is disabled and
    /// means "any valid UTF-8 encoding of any Unicode scalar value" when
    /// Unicode is enabled.
    pub fn dot_matches_new_line(
        &mut self,
        yes: bool,
    ) -> &mut RegexMatcherBuilder {
        self.config.dot_matches_new_line = yes;
        self
    }

    /// Set the value for the greedy swap (`U`) flag.
    ///
    /// When enabled, a pattern like `a*` is lazy (tries to find shortest
    /// match) and `a*?` is greedy (tries to find longest match).
    ///
    /// By default, `a*` is greedy and `a*?` is lazy.
    pub fn swap_greed(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.swap_greed = yes;
        self
    }

    /// Set the value for the ignore whitespace (`x`) flag.
    ///
    /// When enabled, whitespace such as new lines and spaces will be ignored
    /// between expressions of the pattern, and `#` can be used to start a
    /// comment until the next new line.
    pub fn ignore_whitespace(
        &mut self,
        yes: bool,
    ) -> &mut RegexMatcherBuilder {
        self.config.ignore_whitespace = yes;
        self
    }

    /// Set the value for the Unicode (`u`) flag.
    ///
    /// Enabled by default. When disabled, character classes such as `\w` only
    /// match ASCII word characters instead of all Unicode word characters.
    pub fn unicode(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.unicode = yes;
        self
    }

    /// Whether to support octal syntax or not.
    ///
    /// Octal syntax is a little-known way of uttering Unicode codepoints in
    /// a regular expression. For example, `a`, `\x61`, `\u0061` and
    /// `\141` are all equivalent regular expressions, where the last example
    /// shows octal syntax.
    ///
    /// While supporting octal syntax isn't in and of itself a problem, it does
    /// make good error messages harder. That is, in PCRE based regex engines,
    /// syntax like `\0` invokes a backreference, which is explicitly
    /// unsupported in Rust's regex engine. However, many users expect it to
    /// be supported. Therefore, when octal support is disabled, the error
    /// message will explicitly mention that backreferences aren't supported.
    ///
    /// Octal syntax is disabled by default.
    pub fn octal(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.octal = yes;
        self
    }

    /// Set the approximate size limit of the compiled regular expression.
    ///
    /// This roughly corresponds to the number of bytes occupied by a single
    /// compiled program. If the program exceeds this number, then a
    /// compilation error is returned.
    pub fn size_limit(&mut self, bytes: usize) -> &mut RegexMatcherBuilder {
        self.config.size_limit = bytes;
        self
    }

    /// Set the approximate size of the cache used by the DFA.
    ///
    /// This roughly corresponds to the number of bytes that the DFA will
    /// use while searching.
    ///
    /// Note that this is a *per thread* limit. There is no way to set a global
    /// limit. In particular, if a regex is used from multiple threads
    /// simultaneously, then each thread may use up to the number of bytes
    /// specified here.
    pub fn dfa_size_limit(
        &mut self,
        bytes: usize,
    ) -> &mut RegexMatcherBuilder {
        self.config.dfa_size_limit = bytes;
        self
    }

    /// Set the nesting limit for this parser.
    ///
    /// The nesting limit controls how deep the abstract syntax tree is allowed
    /// to be. If the AST exceeds the given limit (e.g., with too many nested
    /// groups), then an error is returned by the parser.
    ///
    /// The purpose of this limit is to act as a heuristic to prevent stack
    /// overflow for consumers that do structural induction on an `Ast` using
    /// explicit recursion. While this crate never does this (instead using
    /// constant stack space and moving the call stack to the heap), other
    /// crates may.
    ///
    /// This limit is not checked until the entire Ast is parsed. Therefore,
    /// if callers want to put a limit on the amount of heap space used, then
    /// they should impose a limit on the length, in bytes, of the concrete
    /// pattern string. In particular, this is viable since this parser
    /// implementation will limit itself to heap space proportional to the
    /// length of the pattern string.
    ///
    /// Note that a nest limit of `0` will return a nest limit error for most
    /// patterns but not all. For example, a nest limit of `0` permits `a` but
    /// not `ab`, since `ab` requires a concatenation, which results in a nest
    /// depth of `1`. In general, a nest limit is not something that manifests
    /// in an obvious way in the concrete syntax, therefore, it should not be
    /// used in a granular way.
    pub fn nest_limit(&mut self, limit: u32) -> &mut RegexMatcherBuilder {
        self.config.nest_limit = limit;
        self
    }

    /// Set an ASCII line terminator for the matcher.
    ///
    /// The purpose of setting a line terminator is to enable a certain class
    /// of optimizations that can make line oriented searching faster. Namely,
    /// when a line terminator is enabled, then the builder will guarantee that
    /// the resulting matcher will never be capable of producing a match that
    /// contains the line terminator. Because of this guarantee, users of the
    /// resulting matcher do not need to slowly execute a search line by line
    /// for line oriented search.
    ///
    /// If the aforementioned guarantee about not matching a line terminator
    /// cannot be made because of how the pattern was written, then the builder
    /// will return an error when attempting to construct the matcher. For
    /// example, the pattern `a\sb` will be transformed such that it can never
    /// match `a\nb` (when `\n` is the line terminator), but the pattern `a\nb`
    /// will result in an error since the `\n` cannot be easily removed without
    /// changing the fundamental intent of the pattern.
    ///
    /// If the given line terminator isn't an ASCII byte (`<=127`), then the
    /// builder will return an error when constructing the matcher.
    pub fn line_terminator(
        &mut self,
        line_term: Option<u8>,
    ) -> &mut RegexMatcherBuilder {
        self.config.line_terminator = line_term.map(LineTerminator::byte);
        self
    }

    /// Set the line terminator to `\r\n` and enable CRLF matching for `$` in
    /// regex patterns.
    ///
    /// This method sets two distinct settings:
    ///
    /// 1. It causes the line terminator for the matcher to be `\r\n`. Namely,
    ///    this prevents the matcher from ever producing a match that contains
    ///    a `\r` or `\n`.
    /// 2. It translates all instances of `$` in the pattern to `(?:\r??$)`.
    ///    This works around the fact that the regex engine does not support
    ///    matching CRLF as a line terminator when using `$`.
    ///
    /// In particular, because of (2), the matches produced by the matcher may
    /// be slightly different than what one would expect given the pattern.
    /// This is the trade off made: in many cases, `$` will "just work" in the
    /// presence of `\r\n` line terminators, but matches may require some
    /// trimming to faithfully represent the intended match.
    ///
    /// Note that if you do not wish to set the line terminator but would still
    /// like `$` to match `\r\n` line terminators, then it is valid to call
    /// `crlf(true)` followed by `line_terminator(None)`. Ordering is
    /// important, since `crlf` and `line_terminator` override each other.
    pub fn crlf(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        if yes {
            self.config.line_terminator = Some(LineTerminator::crlf());
        } else {
            self.config.line_terminator = None;
        }
        self.config.crlf = yes;
        self
    }

    /// Require that all matches occur on word boundaries.
    ///
    /// Enabling this option is subtly different than putting `\b` assertions
    /// on both sides of your pattern. In particular, a `\b` assertion requires
    /// that one side of it match a word character while the other match a
    /// non-word character. This option, in contrast, merely requires that
    /// one side match a non-word character.
    ///
    /// For example, `\b-2\b` will not match `foo -2 bar` since `-` is not a
    /// word character. However, `-2` with this `word` option enabled will
    /// match the `-2` in `foo -2 bar`.
    pub fn word(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.word = yes;
        self
    }
}

/// An implementation of the `Matcher` trait using Rust's standard regex
/// library.
#[derive(Clone, Debug)]
pub struct RegexMatcher {
    /// The configuration specified by the caller.
    config: Config,
    /// The underlying matcher implementation.
    matcher: RegexMatcherImpl,
    /// A regex that never reports false negatives but may report false
    /// positives that is believed to be capable of being matched more quickly
    /// than `regex`. Typically, this is a single literal or an alternation
    /// of literals.
    fast_line_regex: Option<Regex>,
    /// A set of bytes that will never appear in a match.
    non_matching_bytes: ByteSet,
}

impl RegexMatcher {
    /// Create a new matcher from the given pattern using the default
    /// configuration.
    pub fn new(pattern: &str) -> Result<RegexMatcher, Error> {
        RegexMatcherBuilder::new().build(pattern)
    }

    /// Create a new matcher from the given pattern using the default
    /// configuration, but matches lines terminated by `\n`.
    ///
    /// This is meant to be a convenience constructor for using a
    /// `RegexMatcherBuilder` and setting its
    /// [`line_terminator`](struct.RegexMatcherBuilder.html#method.line_terminator)
    /// to `\n`. The purpose of using this constructor is to permit special
    /// optimizations that help speed up line oriented search. These types of
    /// optimizations are only appropriate when matches span no more than one
    /// line. For this reason, this constructor will return an error if the
    /// given pattern contains a literal `\n`. Other uses of `\n` (such as in
    /// `\s`) are removed transparently.
    pub fn new_line_matcher(pattern: &str) -> Result<RegexMatcher, Error> {
        RegexMatcherBuilder::new().line_terminator(Some(b'\n')).build(pattern)
    }
}

/// An encapsulation of the type of matcher we use in `RegexMatcher`.
#[derive(Clone, Debug)]
enum RegexMatcherImpl {
    /// The standard matcher used for all regular expressions.
    Standard(StandardMatcher),
    /// A matcher for an alternation of plain literals.
    MultiLiteral(MultiLiteralMatcher),
    /// A matcher that strips `\r` from the end of matches.
    ///
    /// This is only used when the CRLF hack is enabled and the regex is line
    /// anchored at the end.
    CRLF(CRLFMatcher),
    /// A matcher that only matches at word boundaries. This transforms the
    /// regex to `(^|\W)(...)($|\W)` instead of the more intuitive `\b(...)\b`.
    /// Because of this, the WordMatcher provides its own implementation of
    /// `Matcher` to encapsulate its use of capture groups to make them
    /// invisible to the caller.
    Word(WordMatcher),
}

impl RegexMatcherImpl {
    /// Based on the configuration, create a new implementation of the
    /// `Matcher` trait.
    fn new(expr: &ConfiguredHIR) -> Result<RegexMatcherImpl, Error> {
        if expr.config().word {
            Ok(RegexMatcherImpl::Word(WordMatcher::new(expr)?))
        } else if expr.needs_crlf_stripped() {
            Ok(RegexMatcherImpl::CRLF(CRLFMatcher::new(expr)?))
        } else {
            if let Some(lits) = expr.alternation_literals() {
                if lits.len() >= 40 {
                    let matcher = MultiLiteralMatcher::new(&lits)?;
                    return Ok(RegexMatcherImpl::MultiLiteral(matcher));
                }
            }
            Ok(RegexMatcherImpl::Standard(StandardMatcher::new(expr)?))
        }
    }

    /// Return the underlying regex object used.
    fn regex(&self) -> String {
        match *self {
            RegexMatcherImpl::Word(ref x) => x.regex().to_string(),
            RegexMatcherImpl::CRLF(ref x) => x.regex().to_string(),
            RegexMatcherImpl::MultiLiteral(_) => "<N/A>".to_string(),
            RegexMatcherImpl::Standard(ref x) => x.regex.to_string(),
        }
    }
}

// This implementation just dispatches on the internal matcher impl except
// for the line terminator optimization, which is possibly executed via
// `fast_line_regex`.
impl Matcher for RegexMatcher {
    type Captures = RegexCaptures;
    type Error = NoError;

    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.find_at(haystack, at),
            MultiLiteral(ref m) => m.find_at(haystack, at),
            CRLF(ref m) => m.find_at(haystack, at),
            Word(ref m) => m.find_at(haystack, at),
        }
    }

    fn new_captures(&self) -> Result<RegexCaptures, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.new_captures(),
            MultiLiteral(ref m) => m.new_captures(),
            CRLF(ref m) => m.new_captures(),
            Word(ref m) => m.new_captures(),
        }
    }

    fn capture_count(&self) -> usize {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.capture_count(),
            MultiLiteral(ref m) => m.capture_count(),
            CRLF(ref m) => m.capture_count(),
            Word(ref m) => m.capture_count(),
        }
    }

    fn capture_index(&self, name: &str) -> Option<usize> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.capture_index(name),
            MultiLiteral(ref m) => m.capture_index(name),
            CRLF(ref m) => m.capture_index(name),
            Word(ref m) => m.capture_index(name),
        }
    }

    fn find(&self, haystack: &[u8]) -> Result<Option<Match>, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.find(haystack),
            MultiLiteral(ref m) => m.find(haystack),
            CRLF(ref m) => m.find(haystack),
            Word(ref m) => m.find(haystack),
        }
    }

    fn find_iter<F>(&self, haystack: &[u8], matched: F) -> Result<(), NoError>
    where
        F: FnMut(Match) -> bool,
    {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.find_iter(haystack, matched),
            MultiLiteral(ref m) => m.find_iter(haystack, matched),
            CRLF(ref m) => m.find_iter(haystack, matched),
            Word(ref m) => m.find_iter(haystack, matched),
        }
    }

    fn try_find_iter<F, E>(
        &self,
        haystack: &[u8],
        matched: F,
    ) -> Result<Result<(), E>, NoError>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.try_find_iter(haystack, matched),
            MultiLiteral(ref m) => m.try_find_iter(haystack, matched),
            CRLF(ref m) => m.try_find_iter(haystack, matched),
            Word(ref m) => m.try_find_iter(haystack, matched),
        }
    }

    fn captures(
        &self,
        haystack: &[u8],
        caps: &mut RegexCaptures,
    ) -> Result<bool, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.captures(haystack, caps),
            MultiLiteral(ref m) => m.captures(haystack, caps),
            CRLF(ref m) => m.captures(haystack, caps),
            Word(ref m) => m.captures(haystack, caps),
        }
    }

    fn captures_iter<F>(
        &self,
        haystack: &[u8],
        caps: &mut RegexCaptures,
        matched: F,
    ) -> Result<(), NoError>
    where
        F: FnMut(&RegexCaptures) -> bool,
    {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.captures_iter(haystack, caps, matched),
            MultiLiteral(ref m) => m.captures_iter(haystack, caps, matched),
            CRLF(ref m) => m.captures_iter(haystack, caps, matched),
            Word(ref m) => m.captures_iter(haystack, caps, matched),
        }
    }

    fn try_captures_iter<F, E>(
        &self,
        haystack: &[u8],
        caps: &mut RegexCaptures,
        matched: F,
    ) -> Result<Result<(), E>, NoError>
    where
        F: FnMut(&RegexCaptures) -> Result<bool, E>,
    {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.try_captures_iter(haystack, caps, matched),
            MultiLiteral(ref m) => {
                m.try_captures_iter(haystack, caps, matched)
            }
            CRLF(ref m) => m.try_captures_iter(haystack, caps, matched),
            Word(ref m) => m.try_captures_iter(haystack, caps, matched),
        }
    }

    fn captures_at(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut RegexCaptures,
    ) -> Result<bool, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.captures_at(haystack, at, caps),
            MultiLiteral(ref m) => m.captures_at(haystack, at, caps),
            CRLF(ref m) => m.captures_at(haystack, at, caps),
            Word(ref m) => m.captures_at(haystack, at, caps),
        }
    }

    fn replace<F>(
        &self,
        haystack: &[u8],
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), NoError>
    where
        F: FnMut(Match, &mut Vec<u8>) -> bool,
    {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.replace(haystack, dst, append),
            MultiLiteral(ref m) => m.replace(haystack, dst, append),
            CRLF(ref m) => m.replace(haystack, dst, append),
            Word(ref m) => m.replace(haystack, dst, append),
        }
    }

    fn replace_with_captures<F>(
        &self,
        haystack: &[u8],
        caps: &mut RegexCaptures,
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), NoError>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => {
                m.replace_with_captures(haystack, caps, dst, append)
            }
            MultiLiteral(ref m) => {
                m.replace_with_captures(haystack, caps, dst, append)
            }
            CRLF(ref m) => {
                m.replace_with_captures(haystack, caps, dst, append)
            }
            Word(ref m) => {
                m.replace_with_captures(haystack, caps, dst, append)
            }
        }
    }

    fn is_match(&self, haystack: &[u8]) -> Result<bool, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.is_match(haystack),
            MultiLiteral(ref m) => m.is_match(haystack),
            CRLF(ref m) => m.is_match(haystack),
            Word(ref m) => m.is_match(haystack),
        }
    }

    fn is_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<bool, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.is_match_at(haystack, at),
            MultiLiteral(ref m) => m.is_match_at(haystack, at),
            CRLF(ref m) => m.is_match_at(haystack, at),
            Word(ref m) => m.is_match_at(haystack, at),
        }
    }

    fn shortest_match(
        &self,
        haystack: &[u8],
    ) -> Result<Option<usize>, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.shortest_match(haystack),
            MultiLiteral(ref m) => m.shortest_match(haystack),
            CRLF(ref m) => m.shortest_match(haystack),
            Word(ref m) => m.shortest_match(haystack),
        }
    }

    fn shortest_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<usize>, NoError> {
        use self::RegexMatcherImpl::*;
        match self.matcher {
            Standard(ref m) => m.shortest_match_at(haystack, at),
            MultiLiteral(ref m) => m.shortest_match_at(haystack, at),
            CRLF(ref m) => m.shortest_match_at(haystack, at),
            Word(ref m) => m.shortest_match_at(haystack, at),
        }
    }

    fn non_matching_bytes(&self) -> Option<&ByteSet> {
        Some(&self.non_matching_bytes)
    }

    fn line_terminator(&self) -> Option<LineTerminator> {
        self.config.line_terminator
    }

    fn find_candidate_line(
        &self,
        haystack: &[u8],
    ) -> Result<Option<LineMatchKind>, NoError> {
        Ok(match self.fast_line_regex {
            Some(ref regex) => {
                regex.shortest_match(haystack).map(LineMatchKind::Candidate)
            }
            None => {
                self.shortest_match(haystack)?.map(LineMatchKind::Confirmed)
            }
        })
    }
}

/// The implementation of the standard regex matcher.
#[derive(Clone, Debug)]
struct StandardMatcher {
    /// The regular expression compiled from the pattern provided by the
    /// caller.
    regex: Regex,
    /// A map from capture group name to its corresponding index.
    names: HashMap<String, usize>,
}

impl StandardMatcher {
    fn new(expr: &ConfiguredHIR) -> Result<StandardMatcher, Error> {
        let regex = expr.regex()?;
        let mut names = HashMap::new();
        for (i, optional_name) in regex.capture_names().enumerate() {
            if let Some(name) = optional_name {
                names.insert(name.to_string(), i);
            }
        }
        Ok(StandardMatcher { regex, names })
    }
}

impl Matcher for StandardMatcher {
    type Captures = RegexCaptures;
    type Error = NoError;

    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, NoError> {
        Ok(self
            .regex
            .find_at(haystack, at)
            .map(|m| Match::new(m.start(), m.end())))
    }

    fn new_captures(&self) -> Result<RegexCaptures, NoError> {
        Ok(RegexCaptures::new(self.regex.capture_locations()))
    }

    fn capture_count(&self) -> usize {
        self.regex.captures_len()
    }

    fn capture_index(&self, name: &str) -> Option<usize> {
        self.names.get(name).map(|i| *i)
    }

    fn try_find_iter<F, E>(
        &self,
        haystack: &[u8],
        mut matched: F,
    ) -> Result<Result<(), E>, NoError>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        for m in self.regex.find_iter(haystack) {
            match matched(Match::new(m.start(), m.end())) {
                Ok(true) => continue,
                Ok(false) => return Ok(Ok(())),
                Err(err) => return Ok(Err(err)),
            }
        }
        Ok(Ok(()))
    }

    fn captures_at(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut RegexCaptures,
    ) -> Result<bool, NoError> {
        Ok(self
            .regex
            .captures_read_at(&mut caps.locations_mut(), haystack, at)
            .is_some())
    }

    fn shortest_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<usize>, NoError> {
        Ok(self.regex.shortest_match_at(haystack, at))
    }
}

/// Represents the match offsets of each capturing group in a match.
///
/// The first, or `0`th capture group, always corresponds to the entire match
/// and is guaranteed to be present when a match occurs. The next capture
/// group, at index `1`, corresponds to the first capturing group in the regex,
/// ordered by the position at which the left opening parenthesis occurs.
///
/// Note that not all capturing groups are guaranteed to be present in a match.
/// For example, in the regex, `(?P<foo>\w)|(?P<bar>\W)`, only one of `foo`
/// or `bar` will ever be set in any given match.
///
/// In order to access a capture group by name, you'll need to first find the
/// index of the group using the corresponding matcher's `capture_index`
/// method, and then use that index with `RegexCaptures::get`.
#[derive(Clone, Debug)]
pub struct RegexCaptures(RegexCapturesImp);

#[derive(Clone, Debug)]
enum RegexCapturesImp {
    AhoCorasick {
        /// The start and end of the match, corresponding to capture group 0.
        mat: Option<Match>,
    },
    Regex {
        /// Where the locations are stored.
        locs: CaptureLocations,
        /// These captures behave as if the capturing groups begin at the given
        /// offset. When set to `0`, this has no affect and capture groups are
        /// indexed like normal.
        ///
        /// This is useful when building matchers that wrap arbitrary regular
        /// expressions. For example, `WordMatcher` takes an existing regex
        /// `re` and creates `(?:^|\W)(re)(?:$|\W)`, but hides the fact that
        /// the regex has been wrapped from the caller. In order to do this,
        /// the matcher and the capturing groups must behave as if `(re)` is
        /// the `0`th capture group.
        offset: usize,
        /// When enable, the end of a match has `\r` stripped from it, if one
        /// exists.
        strip_crlf: bool,
    },
}

impl Captures for RegexCaptures {
    fn len(&self) -> usize {
        match self.0 {
            RegexCapturesImp::AhoCorasick { .. } => 1,
            RegexCapturesImp::Regex { ref locs, offset, .. } => {
                locs.len().checked_sub(offset).unwrap()
            }
        }
    }

    fn get(&self, i: usize) -> Option<Match> {
        match self.0 {
            RegexCapturesImp::AhoCorasick { mat, .. } => {
                if i == 0 {
                    mat
                } else {
                    None
                }
            }
            RegexCapturesImp::Regex { ref locs, offset, strip_crlf } => {
                if !strip_crlf {
                    let actual = i.checked_add(offset).unwrap();
                    return locs.pos(actual).map(|(s, e)| Match::new(s, e));
                }

                // currently don't support capture offsetting with CRLF
                // stripping
                assert_eq!(offset, 0);
                let m = match locs.pos(i).map(|(s, e)| Match::new(s, e)) {
                    None => return None,
                    Some(m) => m,
                };
                // If the end position of this match corresponds to the end
                // position of the overall match, then we apply our CRLF
                // stripping. Otherwise, we cannot assume stripping is correct.
                if i == 0 || m.end() == locs.pos(0).unwrap().1 {
                    Some(m.with_end(m.end() - 1))
                } else {
                    Some(m)
                }
            }
        }
    }
}

impl RegexCaptures {
    pub(crate) fn simple() -> RegexCaptures {
        RegexCaptures(RegexCapturesImp::AhoCorasick { mat: None })
    }

    pub(crate) fn new(locs: CaptureLocations) -> RegexCaptures {
        RegexCaptures::with_offset(locs, 0)
    }

    pub(crate) fn with_offset(
        locs: CaptureLocations,
        offset: usize,
    ) -> RegexCaptures {
        RegexCaptures(RegexCapturesImp::Regex {
            locs,
            offset,
            strip_crlf: false,
        })
    }

    pub(crate) fn locations(&self) -> &CaptureLocations {
        match self.0 {
            RegexCapturesImp::AhoCorasick { .. } => {
                panic!("getting locations for simple captures is invalid")
            }
            RegexCapturesImp::Regex { ref locs, .. } => locs,
        }
    }

    pub(crate) fn locations_mut(&mut self) -> &mut CaptureLocations {
        match self.0 {
            RegexCapturesImp::AhoCorasick { .. } => {
                panic!("getting locations for simple captures is invalid")
            }
            RegexCapturesImp::Regex { ref mut locs, .. } => locs,
        }
    }

    pub(crate) fn strip_crlf(&mut self, yes: bool) {
        match self.0 {
            RegexCapturesImp::AhoCorasick { .. } => {
                panic!("setting strip_crlf for simple captures is invalid")
            }
            RegexCapturesImp::Regex { ref mut strip_crlf, .. } => {
                *strip_crlf = yes;
            }
        }
    }

    pub(crate) fn set_simple(&mut self, one: Option<Match>) {
        match self.0 {
            RegexCapturesImp::AhoCorasick { ref mut mat } => {
                *mat = one;
            }
            RegexCapturesImp::Regex { .. } => {
                panic!("setting simple captures for regex is invalid")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grep_matcher::{LineMatchKind, Matcher};

    // Test that enabling word matches does the right thing and demonstrate
    // the difference between it and surrounding the regex in `\b`.
    #[test]
    fn word() {
        let matcher =
            RegexMatcherBuilder::new().word(true).build(r"-2").unwrap();
        assert!(matcher.is_match(b"abc -2 foo").unwrap());

        let matcher =
            RegexMatcherBuilder::new().word(false).build(r"\b-2\b").unwrap();
        assert!(!matcher.is_match(b"abc -2 foo").unwrap());
    }

    // Test that enabling a line terminator prevents it from matching through
    // said line terminator.
    #[test]
    fn line_terminator() {
        // This works, because there's no line terminator specified.
        let matcher = RegexMatcherBuilder::new().build(r"abc\sxyz").unwrap();
        assert!(matcher.is_match(b"abc\nxyz").unwrap());

        // This doesn't.
        let matcher = RegexMatcherBuilder::new()
            .line_terminator(Some(b'\n'))
            .build(r"abc\sxyz")
            .unwrap();
        assert!(!matcher.is_match(b"abc\nxyz").unwrap());
    }

    // Ensure that the builder returns an error if a line terminator is set
    // and the regex could not be modified to remove a line terminator.
    #[test]
    fn line_terminator_error() {
        assert!(RegexMatcherBuilder::new()
            .line_terminator(Some(b'\n'))
            .build(r"a\nz")
            .is_err())
    }

    // Test that enabling CRLF permits `$` to match at the end of a line.
    #[test]
    fn line_terminator_crlf() {
        // Test normal use of `$` with a `\n` line terminator.
        let matcher = RegexMatcherBuilder::new()
            .multi_line(true)
            .build(r"abc$")
            .unwrap();
        assert!(matcher.is_match(b"abc\n").unwrap());

        // Test that `$` doesn't match at `\r\n` boundary normally.
        let matcher = RegexMatcherBuilder::new()
            .multi_line(true)
            .build(r"abc$")
            .unwrap();
        assert!(!matcher.is_match(b"abc\r\n").unwrap());

        // Now check the CRLF handling.
        let matcher = RegexMatcherBuilder::new()
            .multi_line(true)
            .crlf(true)
            .build(r"abc$")
            .unwrap();
        assert!(matcher.is_match(b"abc\r\n").unwrap());
    }

    // Test that smart case works.
    #[test]
    fn case_smart() {
        let matcher =
            RegexMatcherBuilder::new().case_smart(true).build(r"abc").unwrap();
        assert!(matcher.is_match(b"ABC").unwrap());

        let matcher =
            RegexMatcherBuilder::new().case_smart(true).build(r"aBc").unwrap();
        assert!(!matcher.is_match(b"ABC").unwrap());
    }

    // Test that finding candidate lines works as expected.
    #[test]
    fn candidate_lines() {
        fn is_confirmed(m: LineMatchKind) -> bool {
            match m {
                LineMatchKind::Confirmed(_) => true,
                _ => false,
            }
        }
        fn is_candidate(m: LineMatchKind) -> bool {
            match m {
                LineMatchKind::Candidate(_) => true,
                _ => false,
            }
        }

        // With no line terminator set, we can't employ any optimizations,
        // so we get a confirmed match.
        let matcher = RegexMatcherBuilder::new().build(r"\wfoo\s").unwrap();
        let m = matcher.find_candidate_line(b"afoo ").unwrap().unwrap();
        assert!(is_confirmed(m));

        // With a line terminator and a regex specially crafted to have an
        // easy-to-detect inner literal, we can apply an optimization that
        // quickly finds candidate matches.
        let matcher = RegexMatcherBuilder::new()
            .line_terminator(Some(b'\n'))
            .build(r"\wfoo\s")
            .unwrap();
        let m = matcher.find_candidate_line(b"afoo ").unwrap().unwrap();
        assert!(is_candidate(m));
    }
}
