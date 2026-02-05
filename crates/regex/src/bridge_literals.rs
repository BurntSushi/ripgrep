use memchr::memmem;
use regex_syntax::hir::{self, Hir, HirKind};

/// A sequence of literals that must appear in a specific order for a line to qualify as a
/// candidate line.
#[derive(Clone, Debug)]
pub struct LiteralSequence {
    seq: Vec<Vec<u8>>,
    min_required_len: usize,
}

#[derive(Copy, Clone, Debug)]
enum LiteralComponent {
    Char(u8),
    Break,
}

impl LiteralSequence {
    /// Constructs a new `LiteralSequence` from a `Hir`.
    pub fn new(hir: &Hir) -> Option<LiteralSequence> {
        let components = extract_literal_seq_components(hir);

        let mut result = vec![vec![]];
        let mut len = 0usize;
        for comp in components {
            match comp {
                // If we have a character, increase the minimum required length and add the
                // character.
                LiteralComponent::Char(c) => {
                    len += 1;
                    result.last_mut().unwrap().push(c);
                }
                // If we have a break, that means the current literal ended and we have to start a
                // new one.
                LiteralComponent::Break => {
                    // Only start a new literal if the current one is non-empty. Otherwise the
                    // current one can still be used.
                    if !result.last().unwrap().is_empty() {
                        result.push(vec![]);
                    }
                }
            }
        }

        // Get rid of possibly empty literal at the end.
        if result.last().unwrap().is_empty() {
            result.pop();
        }

        let lseq = LiteralSequence { seq: result, min_required_len: len };
        if lseq.is_useful() { Some(lseq) } else { None }
    }

    /// Checks if the literal sequence exists in `haystack`.
    ///
    /// If the literal sequence does exist in the haystack, the position of the last character in
    /// the last literal is returned. Otherwise, `None` is returned.
    pub fn exists_in(&self, haystack: &[u8]) -> Option<usize> {
        if haystack.len() < self.min_required_len {
            return None;
        }
        if haystack.is_empty() {
            return None;
        }
        if self.min_required_len == 0 {
            return Some(0);
        }

        let mut pos = 0;
        for literal in &self.seq {
            match memmem::find(&haystack[pos..], literal) {
                Some(offset) => {
                    pos += offset + literal.len();
                }
                None => {
                    return None;
                }
            }
        }

        Some(pos - 1)
    }

    /// Heuristic for whether using the literal sequence will provide performance improvements, or
    /// at least not significantly reduce the performance.
    fn is_useful(self: &LiteralSequence) -> bool {
        self.seq.len() >= 2
    }
}

fn extract_literal_seq_components(hir: &Hir) -> Vec<LiteralComponent> {
    match hir.kind() {
        HirKind::Capture(cap) => extract_literal_seq_components(&cap.sub),
        HirKind::Look(_) => vec![],
        HirKind::Empty => vec![],
        HirKind::Literal(hir::Literal(bytes)) => {
            bytes.iter().copied().map(LiteralComponent::Char).collect()
        }
        HirKind::Concat(sub_hirs) => {
            sub_hirs.iter().flat_map(extract_literal_seq_components).collect()
        }
        HirKind::Alternation(_) => vec![LiteralComponent::Break],
        HirKind::Class(_) => vec![LiteralComponent::Break],
        HirKind::Repetition(rep) => {
            let mut result = if rep.min == 0 {
                vec![]
            } else {
                extract_literal_seq_components(&rep.sub)
                    .repeat(rep.min as usize)
            };

            // If `rep.max` is strictly greater than `rep.min`, then after repeating the literals
            // obtained from `rep.sub` the minimum amount of times, there will be at least two
            // non-deterministic λ-transitions that follow: one going to the state that consumes
            // one more `rep.sub` expression, and one going to the state that *skips* the
            // `rep.min+1`-th repetition. Just like for other non-deterministic transitions, we
            // need to add a `LiteralComponent::Break` in the sequence.
            //
            // If we don't do this, then expressions like "ab*c" would have the required literals
            // ["ac"], which is incorrect. The correct literals in this case are: ["a", "c"].
            if rep.max.unwrap_or(u32::MAX) > rep.min {
                result.push(LiteralComponent::Break);
            }

            result
        }
    }
}
