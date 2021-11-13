use std::collections::HashMap;

use grep_matcher::{Match, Matcher, NoError};
use regex::bytes::Regex;
use regex_syntax::hir::{self, Hir, HirKind};

use crate::config::ConfiguredHIR;
use crate::error::Error;
use crate::matcher::RegexCaptures;

/// A matcher for implementing "word match" semantics.
#[derive(Clone, Debug)]
pub struct CRLFMatcher {
    /// The regex.
    regex: Regex,
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
        let mut names = HashMap::new();
        for (i, optional_name) in regex.capture_names().enumerate() {
            if let Some(name) = optional_name {
                names.insert(name.to_string(), i.checked_sub(1).unwrap());
            }
        }
        Ok(CRLFMatcher { regex, names })
    }

    /// Return the underlying regex used by this matcher.
    pub fn regex(&self) -> &Regex {
        &self.regex
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
        let m = match self.regex.find_at(haystack, at) {
            None => return Ok(None),
            Some(m) => Match::new(m.start(), m.end()),
        };
        Ok(Some(adjust_match(haystack, m)))
    }

    fn new_captures(&self) -> Result<RegexCaptures, NoError> {
        Ok(RegexCaptures::new(self.regex.capture_locations()))
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
        let r =
            self.regex.captures_read_at(caps.locations_mut(), haystack, at);
        if !r.is_some() {
            return Ok(false);
        }

        // If the end of our match includes a `\r`, then strip it from all
        // capture groups ending at the same location.
        let end = caps.locations().get(0).unwrap().1;
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
        HirKind::Anchor(hir::Anchor::EndLine) => {
            let concat = Hir::concat(vec![
                Hir::repetition(hir::Repetition {
                    kind: hir::RepetitionKind::ZeroOrOne,
                    greedy: false,
                    hir: Box::new(Hir::literal(hir::Literal::Unicode('\r'))),
                }),
                Hir::anchor(hir::Anchor::EndLine),
            ]);
            Hir::group(hir::Group {
                kind: hir::GroupKind::NonCapturing,
                hir: Box::new(concat),
            })
        }
        HirKind::Empty => Hir::empty(),
        HirKind::Literal(x) => Hir::literal(x),
        HirKind::Class(x) => Hir::class(x),
        HirKind::Anchor(x) => Hir::anchor(x),
        HirKind::WordBoundary(x) => Hir::word_boundary(x),
        HirKind::Repetition(mut x) => {
            x.hir = Box::new(crlfify(*x.hir));
            Hir::repetition(x)
        }
        HirKind::Group(mut x) => {
            x.hir = Box::new(crlfify(*x.hir));
            Hir::group(x)
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
        assert_eq!(roundtrip(r"(?m)$$"), "(?:\r??(?m:$))(?:\r??(?m:$))");
        assert_eq!(
            roundtrip(r"(?m)(?:foo$|bar$)"),
            "(?:foo(?:\r??(?m:$))|bar(?:\r??(?m:$)))"
        );
        assert_eq!(roundtrip(r"(?m)$a"), "(?:\r??(?m:$))a");

        // Not a multiline `$`, so no crlfifying occurs.
        assert_eq!(roundtrip(r"$"), "\\z");
        // It's a literal, derp.
        assert_eq!(roundtrip(r"\$"), "\\$");
    }
}
