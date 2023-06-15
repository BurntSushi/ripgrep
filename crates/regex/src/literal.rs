use regex_syntax::hir::Hir;

// BREADCRUMBS:
//
// The way we deal with line terminators in the regex is clunky, but probably
// the least bad option for now unfortunately.
//
// The `non_matching_bytes` routine currently hardcodes line terminators for
// anchors. But it's not really clear it should even care about line terminators
// anyway, since anchors aren't actually part of a match. If we fix that
// though, that currently reveals a different bug elsewhere: '(?-m:^)' isn't
// implemented correctly in multi-line search, because it defers to the fast
// line-by-line strategy, which ends up being wrong. I think the way forward
// there is to:
//
// 1) Adding something in the grep-matcher interface that exposes a way to
// query for \A and \z specifically. If they're in the pattern, then we can
// decide how to handle them.
//
// 2) Perhaps provide a way to "translate \A/\z to ^/$" for cases when
// mulit-line search is not enabled.

#[derive(Clone, Debug)]
pub struct LiteralSets {}

impl LiteralSets {
    /// Create a set of literals from the given HIR expression.
    pub fn new(_: &Hir) -> LiteralSets {
        LiteralSets {}
    }

    /// If it is deemed advantageuous to do so (via various suspicious
    /// heuristics), this will return a single regular expression pattern that
    /// matches a subset of the language matched by the regular expression that
    /// generated these literal sets. The idea here is that the pattern
    /// returned by this method is much cheaper to search for. i.e., It is
    /// usually a single literal or an alternation of literals.
    pub fn one_regex(&self, _word: bool) -> Option<String> {
        None
    }
}
