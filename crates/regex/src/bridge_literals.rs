use memchr::memmem;
use regex_syntax::hir::{self, Hir, HirKind};

/// A sequence of literals that must appear in a specific order for a line to qualify as a
/// candidate line.
#[derive(Clone, Debug, PartialEq)]
pub struct LiteralSequence {
    seq: Vec<Vec<u8>>,
    min_required_len: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum LiteralComponent {
    Char(u8),
    Break,
}

impl LiteralSequence {
    /// Constructs a new `LiteralSequence` from a `Hir`.
    pub fn new(hir: &Hir) -> Option<LiteralSequence> {
        let mut result = Self::from_hir(hir);
        result.min_required_len = std::cmp::max(
            result.min_required_len,
            hir.properties().minimum_len().unwrap_or(0),
        );
        if result.is_useful() { Some(result) } else { None }
    }

    fn from_hir(hir: &Hir) -> LiteralSequence {
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

        LiteralSequence { seq: result, min_required_len: len }
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
        if self.seq.is_empty() {
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
        !self.seq.is_empty()
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
        HirKind::Alternation(sub_hirs) => {
            let sub_results: Vec<Vec<LiteralComponent>> =
                sub_hirs.iter().map(extract_literal_seq_components).collect();

            // An alternation like "(axc)|(ayc)", for example, is equivalent to "a(x|y)c". Based on
            // this idea we extract the common prefix and the common suffix as literal components
            // *outside* of the alternation, which allows us to accumulate more literals.
            let (mut left, right) =
                get_common_prefix_and_suffix(sub_results.as_slice());

            let max_len =
                sub_results.iter().map(|r| r.len()).max().unwrap_or(0);
            // Only insert a break character if at least one of the alternatives is different from
            // the others. An expression like "(abc|abc)", for example, is equivalent to "abc", a
            // literal.
            // This allows us to avoid inserting unnecessary break characters, thus allowing more
            // literals to be extracted.
            if left.len() != max_len {
                push_without_consecutive_break(
                    &mut left,
                    LiteralComponent::Break,
                );
                append_without_consecutive_break(&mut left, &right);
            }

            left
        }
        HirKind::Class(_) => vec![LiteralComponent::Break],
        HirKind::Repetition(rep) => {
            let mut result = if rep.min == 0 {
                vec![]
            } else {
                repeat_without_consecutive_break(
                    &extract_literal_seq_components(&rep.sub),
                    rep.min as usize,
                )
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
                push_without_consecutive_break(
                    &mut result,
                    LiteralComponent::Break,
                );
            }

            result
        }
    }
}

fn push_without_consecutive_break(
    vec: &mut Vec<LiteralComponent>,
    c: LiteralComponent,
) {
    if !(c == LiteralComponent::Break
        && vec.last() == Some(&LiteralComponent::Break))
    {
        vec.push(c)
    }
}

fn append_without_consecutive_break(
    vec: &mut Vec<LiteralComponent>,
    other: &Vec<LiteralComponent>,
) {
    for &c in other {
        push_without_consecutive_break(vec, c);
    }
}

fn repeat_without_consecutive_break(
    vec: &Vec<LiteralComponent>,
    times: usize,
) -> Vec<LiteralComponent> {
    let mut result = Vec::new();
    result.reserve_exact(times * vec.len());

    for _ in 0..times {
        for &c in vec {
            push_without_consecutive_break(&mut result, c);
        }
    }

    result
}

fn get_common_prefix_and_suffix(
    seqs: &[Vec<LiteralComponent>],
) -> (Vec<LiteralComponent>, Vec<LiteralComponent>) {
    if seqs.is_empty() {
        return (vec![], vec![]);
    }

    let left: Vec<LiteralComponent> = seqs[0]
        .iter()
        .copied()
        .enumerate()
        .take_while(|&(i, c)| seqs.iter().all(|seq| seq.get(i) == Some(&c)))
        .map(|(_, c)| c)
        .collect();

    let mut right: Vec<LiteralComponent> = seqs[0]
        .iter()
        .copied()
        .skip(left.len())
        .rev()
        .enumerate()
        .take_while(|&(i, c)| {
            seqs.iter().all(|seq| seq.iter().rev().nth(i) == Some(&c))
        })
        .map(|(_, c)| c)
        .collect();
    right.reverse();

    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex_syntax::hir::Repetition;

    #[test]
    fn extract_literals1() {
        assert_eq!(
            LiteralSequence::from_hir(&Hir::literal("abc".as_bytes())),
            LiteralSequence { seq: vec!["abc".into()], min_required_len: 3 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 0,
                max: None,
                sub: Box::new(Hir::literal("abcde".as_bytes())),
                greedy: false,
            })),
            LiteralSequence { seq: vec![], min_required_len: 0 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 1,
                max: None,
                sub: Box::new(Hir::literal("abcde".as_bytes())),
                greedy: false,
            })),
            LiteralSequence { seq: vec!["abcde".into()], min_required_len: 5 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 5,
                max: Some(5),
                sub: Box::new(Hir::literal("abcde".as_bytes())),
                greedy: false,
            })),
            LiteralSequence {
                seq: vec!["abcdeabcdeabcdeabcdeabcde".into()],
                min_required_len: 25
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 5,
                max: Some(10),
                sub: Box::new(Hir::literal("abcde".as_bytes())),
                greedy: false,
            })),
            LiteralSequence {
                seq: vec!["abcdeabcdeabcdeabcdeabcde".into()],
                min_required_len: 25
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 5,
                max: None,
                sub: Box::new(Hir::literal("abcde".as_bytes())),
                greedy: false,
            })),
            LiteralSequence {
                seq: vec!["abcdeabcdeabcdeabcdeabcde".into()],
                min_required_len: 25
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![Hir::literal(
                "abc".as_bytes()
            ),])),
            LiteralSequence { seq: vec!["abc".into()], min_required_len: 3 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("abc".as_bytes()),
                Hir::literal("abc".as_bytes()),
                Hir::literal("abc".as_bytes()),
            ])),
            LiteralSequence { seq: vec!["abc".into()], min_required_len: 3 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("abcd".as_bytes()),
                Hir::literal("abc".as_bytes()),
                Hir::literal("ab".as_bytes()),
            ])),
            LiteralSequence { seq: vec!["ab".into()], min_required_len: 2 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("abcd".as_bytes()),
                Hir::literal("bcd".as_bytes()),
                Hir::literal("cd".as_bytes()),
            ])),
            LiteralSequence { seq: vec!["cd".into()], min_required_len: 2 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("abcd".as_bytes()),
                Hir::literal("bcd".as_bytes()),
                Hir::literal("cd".as_bytes()),
                Hir::literal("c".as_bytes()),
                Hir::literal("".as_bytes()),
            ])),
            LiteralSequence { seq: vec![], min_required_len: 0 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("abc".as_bytes()),
                Hir::literal("axc".as_bytes()),
            ])),
            LiteralSequence {
                seq: vec!["a".into(), "c".into()],
                min_required_len: 2,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("abc".as_bytes()),
                Hir::literal("axc".as_bytes()),
                Hir::literal("axd".as_bytes()),
            ])),
            LiteralSequence { seq: vec!["a".into()], min_required_len: 1 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("abc".as_bytes()),
                Hir::literal("axc".as_bytes()),
                Hir::literal("vxd".as_bytes()),
            ])),
            LiteralSequence { seq: vec![], min_required_len: 0 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::alternation(vec![
                Hir::literal("how".as_bytes()),
                Hir::literal("cow".as_bytes()),
                Hir::literal("meow".as_bytes()),
            ])),
            LiteralSequence { seq: vec!["ow".into()], min_required_len: 2 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::concat(vec![
                Hir::literal("how".as_bytes()),
                Hir::literal("cow".as_bytes()),
                Hir::literal("meow".as_bytes()),
            ])),
            LiteralSequence {
                seq: vec!["howcowmeow".into()],
                min_required_len: 10,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::concat(vec![
                Hir::literal("hello".as_bytes()),
                Hir::alternation(vec![
                    Hir::literal("how".as_bytes()),
                    Hir::literal("cow".as_bytes()),
                    Hir::literal("meow".as_bytes()),
                ])
            ])),
            LiteralSequence {
                seq: vec!["hello".into(), "ow".into()],
                min_required_len: 7,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::concat(vec![
                Hir::literal("hello".as_bytes()),
                Hir::alternation(vec![
                    Hir::literal("view".as_bytes()),
                    Hir::literal("vinyl".as_bytes()),
                    Hir::literal("video".as_bytes()),
                ])
            ])),
            LiteralSequence {
                seq: vec!["hellovi".into()],
                min_required_len: 7,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::concat(vec![
                Hir::literal("hello".as_bytes()),
                Hir::alternation(vec![
                    Hir::literal("aiew".as_bytes()),
                    Hir::literal("binyl".as_bytes()),
                    Hir::literal("cideo".as_bytes()),
                ])
            ])),
            LiteralSequence { seq: vec!["hello".into()], min_required_len: 5 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::concat(vec![
                Hir::literal("hello".as_bytes()),
                Hir::alternation(vec![
                    Hir::literal("aiyx".as_bytes()),
                    Hir::literal("ainyx".as_bytes()),
                    Hir::literal("aidyx".as_bytes()),
                ])
            ])),
            LiteralSequence {
                seq: vec!["helloai".into(), "yx".into()],
                min_required_len: 9,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 5,
                max: Some(5),
                sub: Box::new(Hir::alternation(vec![
                    Hir::literal("abc".as_bytes()),
                    Hir::literal("def".as_bytes()),
                ])),
                greedy: false,
            })),
            LiteralSequence { seq: vec![], min_required_len: 0 }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 3,
                max: Some(5),
                sub: Box::new(Hir::alternation(vec![
                    Hir::literal("abc".as_bytes()),
                    Hir::literal("axc".as_bytes()),
                ])),
                greedy: false,
            })),
            LiteralSequence {
                seq: vec!["a".into(), "ca".into(), "ca".into(), "c".into()],
                min_required_len: 6,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::concat(vec![
                Hir::literal("x".as_bytes()),
                Hir::alternation(vec![
                    Hir::literal("ab".as_bytes()),
                    Hir::literal("b".as_bytes()),
                ]),
                Hir::literal("y".as_bytes()),
            ])),
            LiteralSequence {
                seq: vec!["x".into(), "by".into()],
                min_required_len: 3,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 3,
                max: Some(5),
                sub: Box::new(Hir::concat(vec![
                    Hir::literal("x".as_bytes()),
                    Hir::alternation(vec![
                        Hir::literal("ab".as_bytes()),
                        Hir::literal("b".as_bytes()),
                    ]),
                    Hir::literal("y".as_bytes()),
                ])),
                greedy: false,
            })),
            LiteralSequence {
                seq: vec!["x".into(), "byx".into(), "byx".into(), "by".into()],
                min_required_len: 9,
            }
        );
        assert_eq!(
            LiteralSequence::from_hir(&Hir::repetition(Repetition {
                min: 3,
                max: Some(5),
                sub: Box::new(Hir::concat(vec![
                    Hir::literal("x".as_bytes()),
                    Hir::alternation(vec![
                        Hir::literal("ab".as_bytes()),
                        Hir::literal("a".as_bytes()),
                    ]),
                    Hir::literal("y".as_bytes()),
                ])),
                greedy: false,
            })),
            LiteralSequence {
                seq: vec!["xa".into(), "yxa".into(), "yxa".into(), "y".into()],
                min_required_len: 9,
            }
        );
    }
}
