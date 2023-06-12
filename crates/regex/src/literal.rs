use regex_syntax::hir::Hir;

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
