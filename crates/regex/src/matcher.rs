use {
    grep_matcher::{
        ByteSet, Captures, LineMatchKind, LineTerminator, Match, Matcher,
        NoError,
    },
    regex_automata::{
        meta::Regex, util::captures::Captures as AutomataCaptures, Input,
        PatternID,
    },
};

use crate::{config::Config, error::Error, literal::InnerLiterals};

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
        self.build_many(&[pattern])
    }

    /// Build a new matcher using the current configuration for the provided
    /// patterns. The resulting matcher behaves as if all of the patterns
    /// given are joined together into a single alternation. That is, it
    /// reports matches where at least one of the given patterns matches.
    pub fn build_many<P: AsRef<str>>(
        &self,
        patterns: &[P],
    ) -> Result<RegexMatcher, Error> {
        let mut chir = self.config.build_many(patterns)?;
        // 'whole_line' is a strict subset of 'word', so when it is enabled,
        // we don't need to both with any specific to word matching.
        if chir.config().whole_line {
            chir = chir.into_whole_line();
        } else if chir.config().word {
            chir = chir.into_word();
        }
        let regex = chir.to_regex()?;
        log::trace!("final regex: {:?}", chir.hir().to_string());

        let non_matching_bytes = chir.non_matching_bytes();
        // If we can pick out some literals from the regex, then we might be
        // able to build a faster regex that quickly identifies candidate
        // matching lines. The regex engine will do what it can on its own, but
        // we can specifically do a little more when a line terminator is set.
        // For example, for a regex like `\w+foo\w+`, we can look for `foo`,
        // and when a match is found, look for the line containing `foo` and
        // then run the original regex on only that line. (In this case, the
        // regex engine is likely to handle this case for us since it's so
        // simple, but the idea applies.)
        let fast_line_regex = InnerLiterals::new(&chir, &regex).one_regex()?;

        // We override the line terminator in case the configured HIR doesn't
        // support it.
        let mut config = self.config.clone();
        config.line_terminator = chir.line_terminator();
        Ok(RegexMatcher { config, regex, fast_line_regex, non_matching_bytes })
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
        self.build_many(literals)
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
    /// 2. It enables CRLF mode for `^` and `$`. This means that line anchors
    ///    will treat both `\r` and `\n` as line terminators, but will never
    ///    match between a `\r` and `\n`.
    ///
    /// Note that if you do not wish to set the line terminator but would
    /// still like `$` to match `\r\n` line terminators, then it is valid to
    /// call `crlf(true)` followed by `line_terminator(None)`. Ordering is
    /// important, since `crlf` sets the line terminator, but `line_terminator`
    /// does not touch the `crlf` setting.
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

    /// Whether the patterns should be treated as literal strings or not. When
    /// this is active, all characters, including ones that would normally be
    /// special regex meta characters, are matched literally.
    pub fn fixed_strings(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.fixed_strings = yes;
        self
    }

    /// Whether each pattern should match the entire line or not. This is
    /// equivalent to surrounding the pattern with `(?m:^)` and `(?m:$)`.
    pub fn whole_line(&mut self, yes: bool) -> &mut RegexMatcherBuilder {
        self.config.whole_line = yes;
        self
    }
}

/// An implementation of the `Matcher` trait using Rust's standard regex
/// library.
#[derive(Clone, Debug)]
pub struct RegexMatcher {
    /// The configuration specified by the caller.
    config: Config,
    /// The regular expression compiled from the pattern provided by the
    /// caller.
    regex: Regex,
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
    /// This is meant to be a convenience constructor for
    /// using a `RegexMatcherBuilder` and setting its
    /// [`line_terminator`](RegexMatcherBuilder::method.line_terminator) to
    /// `\n`. The purpose of using this constructor is to permit special
    /// optimizations that help speed up line oriented search. These types of
    /// optimizations are only appropriate when matches span no more than one
    /// line. For this reason, this constructor will return an error if the
    /// given pattern contains a literal `\n`. Other uses of `\n` (such as in
    /// `\s`) are removed transparently.
    pub fn new_line_matcher(pattern: &str) -> Result<RegexMatcher, Error> {
        RegexMatcherBuilder::new().line_terminator(Some(b'\n')).build(pattern)
    }
}

// This implementation just dispatches on the internal matcher impl except
// for the line terminator optimization, which is possibly executed via
// `fast_line_regex`.
impl Matcher for RegexMatcher {
    type Captures = RegexCaptures;
    type Error = NoError;

    #[inline]
    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, NoError> {
        let input = Input::new(haystack).span(at..haystack.len());
        Ok(self.regex.find(input).map(|m| Match::new(m.start(), m.end())))
    }

    #[inline]
    fn new_captures(&self) -> Result<RegexCaptures, NoError> {
        Ok(RegexCaptures::new(self.regex.create_captures()))
    }

    #[inline]
    fn capture_count(&self) -> usize {
        self.regex.captures_len()
    }

    #[inline]
    fn capture_index(&self, name: &str) -> Option<usize> {
        self.regex.group_info().to_index(PatternID::ZERO, name)
    }

    #[inline]
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

    #[inline]
    fn captures_at(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut RegexCaptures,
    ) -> Result<bool, NoError> {
        let input = Input::new(haystack).span(at..haystack.len());
        let caps = caps.captures_mut();
        self.regex.search_captures(&input, caps);
        Ok(caps.is_match())
    }

    #[inline]
    fn shortest_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<usize>, NoError> {
        let input = Input::new(haystack).span(at..haystack.len());
        Ok(self.regex.search_half(&input).map(|hm| hm.offset()))
    }

    #[inline]
    fn non_matching_bytes(&self) -> Option<&ByteSet> {
        Some(&self.non_matching_bytes)
    }

    #[inline]
    fn line_terminator(&self) -> Option<LineTerminator> {
        self.config.line_terminator
    }

    #[inline]
    fn find_candidate_line(
        &self,
        haystack: &[u8],
    ) -> Result<Option<LineMatchKind>, NoError> {
        Ok(match self.fast_line_regex {
            Some(ref regex) => {
                let input = Input::new(haystack);
                regex
                    .search_half(&input)
                    .map(|hm| LineMatchKind::Candidate(hm.offset()))
            }
            None => {
                self.shortest_match(haystack)?.map(LineMatchKind::Confirmed)
            }
        })
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
pub struct RegexCaptures {
    /// Where the captures are stored.
    caps: AutomataCaptures,
}

impl Captures for RegexCaptures {
    #[inline]
    fn len(&self) -> usize {
        self.caps.group_info().all_group_len()
    }

    #[inline]
    fn get(&self, i: usize) -> Option<Match> {
        self.caps.get_group(i).map(|sp| Match::new(sp.start, sp.end))
    }
}

impl RegexCaptures {
    #[inline]
    pub(crate) fn new(caps: AutomataCaptures) -> RegexCaptures {
        RegexCaptures { caps }
    }

    #[inline]
    pub(crate) fn captures_mut(&mut self) -> &mut AutomataCaptures {
        &mut self.caps
    }
}

#[cfg(test)]
mod tests {
    use grep_matcher::{LineMatchKind, Matcher};

    use super::*;

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
    // FIXME: Re-enable this test once inner literal extraction works.
    #[test]
    #[ignore]
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
