use std::collections::HashMap;

use {
    grep_matcher::{Match, Matcher, NoError},
    regex_automata::{meta::Regex, Input, PatternID},
    regex_syntax::hir::{self, Hir, HirKind},
};

use crate::{config::ConfiguredHIR, error::Error, matcher::RegexCaptures};

/// A matcher for implementing "word match" semantics.
#[derive(Clone, Debug)]
pub struct CRLFMatcher {
    /// The regex.
    regex: Regex,
    /// The pattern string corresponding to the regex above.
    pattern: String,
    /// A map from capture group name to capture group index.
    names: HashMap<String, usize>,
}

impl CRLFMatcher {
    /// Create a new matcher from the given pattern that strips `\r` from the
    /// end of every match.
    ///
    /// This panics if the given expression doesn't need its CRLF stripped.
    pub fn new(expr: &ConfiguredHIR) -> Result<CRLFMatcher, Error> {
        assert!(expr.needs_crlf_stripped());

        let regex = expr.regex()?;
        let pattern = expr.pattern();
        let mut names = HashMap::new();
        let it = regex.group_info().pattern_names(PatternID::ZERO);
        for (i, optional_name) in it.enumerate() {
            if let Some(name) = optional_name {
                names.insert(name.to_string(), i.checked_sub(1).unwrap());
            }
        }
        Ok(CRLFMatcher { regex, pattern, names })
    }

    /// Return the underlying pattern string for the regex used by this
    /// matcher.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

impl Matcher for CRLFMatcher {
    type Captures = RegexCaptures;
    type Error = NoError;

    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, NoError> {
        let input = Input::new(haystack).span(at..haystack.len());
        let m = match self.regex.find(input) {
            None => return Ok(None),
            Some(m) => Match::new(m.start(), m.end()),
        };
        Ok(Some(adjust_match(haystack, m)))
    }

    fn new_captures(&self) -> Result<RegexCaptures, NoError> {
        Ok(RegexCaptures::new(self.regex.create_captures()))
    }

    fn capture_count(&self) -> usize {
        self.regex.captures_len().checked_sub(1).unwrap()
    }

    fn capture_index(&self, name: &str) -> Option<usize> {
        self.names.get(name).map(|i| *i)
    }

    fn captures_at(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut RegexCaptures,
    ) -> Result<bool, NoError> {
        caps.strip_crlf(false);
        let input = Input::new(haystack).span(at..haystack.len());
        self.regex.search_captures(&input, caps.locations_mut());
        if !caps.locations().is_match() {
            return Ok(false);
        }

        // If the end of our match includes a `\r`, then strip it from all
        // capture groups ending at the same location.
        let end = caps.locations().get_match().unwrap().end();
        if end > 0 && haystack.get(end - 1) == Some(&b'\r') {
            caps.strip_crlf(true);
        }
        Ok(true)
    }

    // We specifically do not implement other methods like find_iter or
    // captures_iter. Namely, the iter methods are guaranteed to be correct
    // by virtue of implementing find_at and captures_at above.
}

/// If the given match ends with a `\r`, then return a new match that ends
/// immediately before the `\r`.
pub fn adjust_match(haystack: &[u8], m: Match) -> Match {
    if m.end() > 0 && haystack.get(m.end() - 1) == Some(&b'\r') {
        m.with_end(m.end() - 1)
    } else {
        m
    }
}

/// Substitutes all occurrences of multi-line enabled `$` with `(?:\r?$)`.
///
/// This does not preserve the exact semantics of the given expression,
/// however, it does have the useful property that anything that matched the
/// given expression will also match the returned expression. The difference is
/// that the returned expression can match possibly other things as well.
///
/// The principle reason why we do this is because the underlying regex engine
/// doesn't support CRLF aware `$` look-around. It's planned to fix it at that
/// level, but we perform this kludge in the mean time.
///
/// Note that while the match preserving semantics are nice and neat, the
/// match position semantics are quite a bit messier. Namely, `$` only ever
/// matches the position between characters where as `\r??` can match a
/// character and change the offset. This is regretable, but works out pretty
/// nicely in most cases, especially when a match is limited to a single line.
pub fn crlfify(expr: Hir) -> Hir {
    match expr.into_kind() {
        HirKind::Look(hir::Look::EndLF) => Hir::concat(vec![
            Hir::repetition(hir::Repetition {
                min: 0,
                max: Some(1),
                greedy: false,
                sub: Box::new(Hir::literal("\r".as_bytes())),
            }),
            Hir::look(hir::Look::EndLF),
        ]),
        HirKind::Empty => Hir::empty(),
        HirKind::Literal(hir::Literal(x)) => Hir::literal(x),
        HirKind::Class(x) => Hir::class(x),
        HirKind::Look(x) => Hir::look(x),
        HirKind::Repetition(mut x) => {
            x.sub = Box::new(crlfify(*x.sub));
            Hir::repetition(x)
        }
        HirKind::Capture(mut x) => {
            x.sub = Box::new(crlfify(*x.sub));
            Hir::capture(x)
        }
        HirKind::Concat(xs) => {
            Hir::concat(xs.into_iter().map(crlfify).collect())
        }
        HirKind::Alternation(xs) => {
            Hir::alternation(xs.into_iter().map(crlfify).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::crlfify;
    use regex_syntax::Parser;

    fn roundtrip(pattern: &str) -> String {
        let expr1 = Parser::new().parse(pattern).unwrap();
        let expr2 = crlfify(expr1);
        expr2.to_string()
    }

    #[test]
    fn various() {
        assert_eq!(roundtrip(r"(?m)$"), "(?:\r??(?m:$))");
        assert_eq!(roundtrip(r"(?m)$$"), "(?:\r??(?m:$)\r??(?m:$))");
        assert_eq!(
            roundtrip(r"(?m)(?:foo$|bar$)"),
            "(?:(?:(?:foo)\r??(?m:$))|(?:(?:bar)\r??(?m:$)))"
        );
        assert_eq!(roundtrip(r"(?m)$a"), "(?:\r??(?m:$)a)");

        // Not a multiline `$`, so no crlfifying occurs.
        assert_eq!(roundtrip(r"$"), "\\z");
        // It's a literal, derp.
        assert_eq!(roundtrip(r"\$"), "\\$");
    }
}
