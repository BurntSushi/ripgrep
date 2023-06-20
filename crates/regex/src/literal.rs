#![allow(warnings)]

use {
    regex_automata::meta::Regex,
    regex_syntax::hir::{
        literal::{self, Seq},
        Hir,
    },
};

use crate::config::ConfiguredHIR;

#[derive(Clone, Debug)]
pub(crate) struct InnerLiterals {
    seq: Seq,
}

impl InnerLiterals {
    /// Create a set of inner literals from the given HIR expression.
    ///
    /// If no line terminator was configured, then this always declines to
    /// extract literals because the inner literal optimization may not be
    /// valid.
    ///
    /// Note that this requires the actual regex that will be used for a search
    /// because it will query some state about the compiled regex. That state
    /// may influence inner literal extraction.
    pub(crate) fn new(chir: &ConfiguredHIR, re: &Regex) -> InnerLiterals {
        let seq = Seq::infinite();
        if chir.config().line_terminator.is_none() || re.is_accelerated() {
            return InnerLiterals::none();
        }
        InnerLiterals { seq }
    }

    /// Returns a infinite set of inner literals, such that it can never
    /// produce a matcher.
    pub(crate) fn none() -> InnerLiterals {
        InnerLiterals { seq: Seq::infinite() }
    }

    /// If it is deemed advantageuous to do so (via various suspicious
    /// heuristics), this will return a single regular expression pattern that
    /// matches a subset of the language matched by the regular expression that
    /// generated these literal sets. The idea here is that the pattern
    /// returned by this method is much cheaper to search for. i.e., It is
    /// usually a single literal or an alternation of literals.
    pub(crate) fn one_regex(&self) -> Option<String> {
        if !self.seq.is_finite() {
            return None;
        }
        None
    }
}
