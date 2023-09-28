/*!
A collection of routines for performing operations on lines.
*/

use {
    bstr::ByteSlice,
    grep_matcher::{LineTerminator, Match},
};

/// An iterator over lines in a particular slice of bytes.
///
/// Line terminators are considered part of the line they terminate. All lines
/// yielded by the iterator are guaranteed to be non-empty.
///
/// `'b` refers to the lifetime of the underlying bytes.
#[derive(Debug)]
pub struct LineIter<'b> {
    bytes: &'b [u8],
    stepper: LineStep,
}

impl<'b> LineIter<'b> {
    /// Create a new line iterator that yields lines in the given bytes that
    /// are terminated by `line_term`.
    pub fn new(line_term: u8, bytes: &'b [u8]) -> LineIter<'b> {
        let stepper = LineStep::new(line_term, 0, bytes.len());
        LineIter { bytes, stepper }
    }
}

impl<'b> Iterator for LineIter<'b> {
    type Item = &'b [u8];

    fn next(&mut self) -> Option<&'b [u8]> {
        self.stepper.next_match(self.bytes).map(|m| &self.bytes[m])
    }
}

/// An explicit iterator over lines in a particular slice of bytes.
///
/// This iterator avoids borrowing the bytes themselves, and instead requires
/// callers to explicitly provide the bytes when moving through the iterator.
/// While not idiomatic, this provides a simple way of iterating over lines
/// that doesn't require borrowing the slice itself, which can be convenient.
///
/// Line terminators are considered part of the line they terminate. All lines
/// yielded by the iterator are guaranteed to be non-empty.
#[derive(Debug)]
pub struct LineStep {
    line_term: u8,
    pos: usize,
    end: usize,
}

impl LineStep {
    /// Create a new line iterator over the given range of bytes using the
    /// given line terminator.
    ///
    /// Callers should provide the actual bytes for each call to `next`. The
    /// same slice must be provided to each call.
    ///
    /// This panics if `start` is not less than or equal to `end`.
    pub fn new(line_term: u8, start: usize, end: usize) -> LineStep {
        LineStep { line_term, pos: start, end }
    }

    /// Return the start and end position of the next line in the given bytes.
    ///
    /// The caller must past exactly the same slice of bytes for each call to
    /// `next`.
    ///
    /// The range returned includes the line terminator. Ranges are always
    /// non-empty.
    pub fn next(&mut self, bytes: &[u8]) -> Option<(usize, usize)> {
        self.next_impl(bytes)
    }

    /// Like next, but returns a `Match` instead of a tuple.
    #[inline(always)]
    pub(crate) fn next_match(&mut self, bytes: &[u8]) -> Option<Match> {
        self.next_impl(bytes).map(|(s, e)| Match::new(s, e))
    }

    #[inline(always)]
    fn next_impl(&mut self, mut bytes: &[u8]) -> Option<(usize, usize)> {
        bytes = &bytes[..self.end];
        match bytes[self.pos..].find_byte(self.line_term) {
            None => {
                if self.pos < bytes.len() {
                    let m = (self.pos, bytes.len());
                    assert!(m.0 <= m.1);

                    self.pos = m.1;
                    Some(m)
                } else {
                    None
                }
            }
            Some(line_end) => {
                let m = (self.pos, self.pos + line_end + 1);
                assert!(m.0 <= m.1);

                self.pos = m.1;
                Some(m)
            }
        }
    }
}

/// Count the number of occurrences of `line_term` in `bytes`.
pub(crate) fn count(bytes: &[u8], line_term: u8) -> u64 {
    memchr::memchr_iter(line_term, bytes).count() as u64
}

/// Given a line that possibly ends with a terminator, return that line without
/// the terminator.
#[inline(always)]
pub(crate) fn without_terminator(
    bytes: &[u8],
    line_term: LineTerminator,
) -> &[u8] {
    let line_term = line_term.as_bytes();
    let start = bytes.len().saturating_sub(line_term.len());
    if bytes.get(start..) == Some(line_term) {
        return &bytes[..bytes.len() - line_term.len()];
    }
    bytes
}

/// Return the start and end offsets of the lines containing the given range
/// of bytes.
///
/// Line terminators are considered part of the line they terminate.
#[inline(always)]
pub(crate) fn locate(bytes: &[u8], line_term: u8, range: Match) -> Match {
    let line_start =
        bytes[..range.start()].rfind_byte(line_term).map_or(0, |i| i + 1);
    let line_end =
        if range.end() > line_start && bytes[range.end() - 1] == line_term {
            range.end()
        } else {
            bytes[range.end()..]
                .find_byte(line_term)
                .map_or(bytes.len(), |i| range.end() + i + 1)
        };
    Match::new(line_start, line_end)
}

/// Returns the minimal starting offset of the line that occurs `count` lines
/// before the last line in `bytes`.
///
/// Lines are terminated by `line_term`. If `count` is zero, then this returns
/// the starting offset of the last line in `bytes`.
///
/// If `bytes` ends with a line terminator, then the terminator itself is
/// considered part of the last line.
pub(crate) fn preceding(bytes: &[u8], line_term: u8, count: usize) -> usize {
    preceding_by_pos(bytes, bytes.len(), line_term, count)
}

/// Returns the minimal starting offset of the line that occurs `count` lines
/// before the line containing `pos`. Lines are terminated by `line_term`.
/// If `count` is zero, then this returns the starting offset of the line
/// containing `pos`.
///
/// If `pos` points just past a line terminator, then it is considered part of
/// the line that it terminates. For example, given `bytes = b"abc\nxyz\n"`
/// and `pos = 7`, `preceding(bytes, pos, b'\n', 0)` returns `4` (as does `pos
/// = 8`) and `preceding(bytes, pos, `b'\n', 1)` returns `0`.
fn preceding_by_pos(
    bytes: &[u8],
    mut pos: usize,
    line_term: u8,
    mut count: usize,
) -> usize {
    if pos == 0 {
        return 0;
    } else if bytes[pos - 1] == line_term {
        pos -= 1;
    }
    loop {
        match bytes[..pos].rfind_byte(line_term) {
            None => {
                return 0;
            }
            Some(i) => {
                if count == 0 {
                    return i + 1;
                } else if i == 0 {
                    return 0;
                }
                count -= 1;
                pos = i;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use grep_matcher::Match;

    use super::*;

    const SHERLOCK: &'static str = "\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
can extract a clew from a wisp of straw or a flake of cigar ash;
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.\
";

    fn m(start: usize, end: usize) -> Match {
        Match::new(start, end)
    }

    fn lines(text: &str) -> Vec<&str> {
        let mut results = vec![];
        let mut it = LineStep::new(b'\n', 0, text.len());
        while let Some(m) = it.next_match(text.as_bytes()) {
            results.push(&text[m]);
        }
        results
    }

    fn line_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
        let mut results = vec![];
        let mut it = LineStep::new(b'\n', 0, text.len());
        while let Some(m) = it.next_match(text.as_bytes()) {
            results.push(m.start()..m.end());
        }
        results
    }

    fn prev(text: &str, pos: usize, count: usize) -> usize {
        preceding_by_pos(text.as_bytes(), pos, b'\n', count)
    }

    fn loc(text: &str, start: usize, end: usize) -> Match {
        locate(text.as_bytes(), b'\n', Match::new(start, end))
    }

    #[test]
    fn line_count() {
        assert_eq!(0, count(b"", b'\n'));
        assert_eq!(1, count(b"\n", b'\n'));
        assert_eq!(2, count(b"\n\n", b'\n'));
        assert_eq!(2, count(b"a\nb\nc", b'\n'));
    }

    #[test]
    fn line_locate() {
        let t = SHERLOCK;
        let lines = line_ranges(t);

        assert_eq!(
            loc(t, lines[0].start, lines[0].end),
            m(lines[0].start, lines[0].end)
        );
        assert_eq!(
            loc(t, lines[0].start + 1, lines[0].end),
            m(lines[0].start, lines[0].end)
        );
        assert_eq!(
            loc(t, lines[0].end - 1, lines[0].end),
            m(lines[0].start, lines[0].end)
        );
        assert_eq!(
            loc(t, lines[0].end, lines[0].end),
            m(lines[1].start, lines[1].end)
        );

        assert_eq!(
            loc(t, lines[5].start, lines[5].end),
            m(lines[5].start, lines[5].end)
        );
        assert_eq!(
            loc(t, lines[5].start + 1, lines[5].end),
            m(lines[5].start, lines[5].end)
        );
        assert_eq!(
            loc(t, lines[5].end - 1, lines[5].end),
            m(lines[5].start, lines[5].end)
        );
        assert_eq!(
            loc(t, lines[5].end, lines[5].end),
            m(lines[5].start, lines[5].end)
        );
    }

    #[test]
    fn line_locate_weird() {
        assert_eq!(loc("", 0, 0), m(0, 0));

        assert_eq!(loc("\n", 0, 1), m(0, 1));
        assert_eq!(loc("\n", 1, 1), m(1, 1));

        assert_eq!(loc("\n\n", 0, 0), m(0, 1));
        assert_eq!(loc("\n\n", 0, 1), m(0, 1));
        assert_eq!(loc("\n\n", 1, 1), m(1, 2));
        assert_eq!(loc("\n\n", 1, 2), m(1, 2));
        assert_eq!(loc("\n\n", 2, 2), m(2, 2));

        assert_eq!(loc("a\nb\nc", 0, 1), m(0, 2));
        assert_eq!(loc("a\nb\nc", 1, 2), m(0, 2));
        assert_eq!(loc("a\nb\nc", 2, 3), m(2, 4));
        assert_eq!(loc("a\nb\nc", 3, 4), m(2, 4));
        assert_eq!(loc("a\nb\nc", 4, 5), m(4, 5));
        assert_eq!(loc("a\nb\nc", 5, 5), m(4, 5));
    }

    #[test]
    fn line_iter() {
        assert_eq!(lines("abc"), vec!["abc"]);

        assert_eq!(lines("abc\n"), vec!["abc\n"]);
        assert_eq!(lines("abc\nxyz"), vec!["abc\n", "xyz"]);
        assert_eq!(lines("abc\nxyz\n"), vec!["abc\n", "xyz\n"]);

        assert_eq!(lines("abc\n\n"), vec!["abc\n", "\n"]);
        assert_eq!(lines("abc\n\n\n"), vec!["abc\n", "\n", "\n"]);
        assert_eq!(lines("abc\n\nxyz"), vec!["abc\n", "\n", "xyz"]);
        assert_eq!(lines("abc\n\nxyz\n"), vec!["abc\n", "\n", "xyz\n"]);
        assert_eq!(lines("abc\nxyz\n\n"), vec!["abc\n", "xyz\n", "\n"]);

        assert_eq!(lines("\n"), vec!["\n"]);
        assert_eq!(lines(""), Vec::<&str>::new());
    }

    #[test]
    fn line_iter_empty() {
        let mut it = LineStep::new(b'\n', 0, 0);
        assert_eq!(it.next(b"abc"), None);
    }

    #[test]
    fn preceding_lines_doc() {
        // These are the examples mentions in the documentation of `preceding`.
        let bytes = b"abc\nxyz\n";
        assert_eq!(4, preceding_by_pos(bytes, 7, b'\n', 0));
        assert_eq!(4, preceding_by_pos(bytes, 8, b'\n', 0));
        assert_eq!(0, preceding_by_pos(bytes, 7, b'\n', 1));
        assert_eq!(0, preceding_by_pos(bytes, 8, b'\n', 1));
    }

    #[test]
    fn preceding_lines_sherlock() {
        let t = SHERLOCK;
        let lines = line_ranges(t);

        // The following tests check the count == 0 case, i.e., finding the
        // beginning of the line containing the given position.
        assert_eq!(0, prev(t, 0, 0));
        assert_eq!(0, prev(t, 1, 0));
        // The line terminator is addressed by `end-1` and terminates the line
        // it is part of.
        assert_eq!(0, prev(t, lines[0].end - 1, 0));
        assert_eq!(lines[0].start, prev(t, lines[0].end, 0));
        // The end position of line addresses the byte immediately following a
        // line terminator, which puts it on the following line.
        assert_eq!(lines[1].start, prev(t, lines[0].end + 1, 0));

        // Now tests for count > 0.
        assert_eq!(0, prev(t, 0, 1));
        assert_eq!(0, prev(t, 0, 2));
        assert_eq!(0, prev(t, 1, 1));
        assert_eq!(0, prev(t, 1, 2));
        assert_eq!(0, prev(t, lines[0].end - 1, 1));
        assert_eq!(0, prev(t, lines[0].end - 1, 2));
        assert_eq!(0, prev(t, lines[0].end, 1));
        assert_eq!(0, prev(t, lines[0].end, 2));
        assert_eq!(lines[3].start, prev(t, lines[4].end - 1, 1));
        assert_eq!(lines[3].start, prev(t, lines[4].end, 1));
        assert_eq!(lines[4].start, prev(t, lines[4].end + 1, 1));

        // The last line has no line terminator.
        assert_eq!(lines[5].start, prev(t, lines[5].end, 0));
        assert_eq!(lines[5].start, prev(t, lines[5].end - 1, 0));
        assert_eq!(lines[4].start, prev(t, lines[5].end, 1));
        assert_eq!(lines[0].start, prev(t, lines[5].end, 5));
    }

    #[test]
    fn preceding_lines_short() {
        let t = "a\nb\nc\nd\ne\nf\n";
        let lines = line_ranges(t);
        assert_eq!(12, t.len());

        assert_eq!(lines[5].start, prev(t, lines[5].end, 0));
        assert_eq!(lines[4].start, prev(t, lines[5].end, 1));
        assert_eq!(lines[3].start, prev(t, lines[5].end, 2));
        assert_eq!(lines[2].start, prev(t, lines[5].end, 3));
        assert_eq!(lines[1].start, prev(t, lines[5].end, 4));
        assert_eq!(lines[0].start, prev(t, lines[5].end, 5));
        assert_eq!(lines[0].start, prev(t, lines[5].end, 6));

        assert_eq!(lines[5].start, prev(t, lines[5].end - 1, 0));
        assert_eq!(lines[4].start, prev(t, lines[5].end - 1, 1));
        assert_eq!(lines[3].start, prev(t, lines[5].end - 1, 2));
        assert_eq!(lines[2].start, prev(t, lines[5].end - 1, 3));
        assert_eq!(lines[1].start, prev(t, lines[5].end - 1, 4));
        assert_eq!(lines[0].start, prev(t, lines[5].end - 1, 5));
        assert_eq!(lines[0].start, prev(t, lines[5].end - 1, 6));

        assert_eq!(lines[4].start, prev(t, lines[5].start, 0));
        assert_eq!(lines[3].start, prev(t, lines[5].start, 1));
        assert_eq!(lines[2].start, prev(t, lines[5].start, 2));
        assert_eq!(lines[1].start, prev(t, lines[5].start, 3));
        assert_eq!(lines[0].start, prev(t, lines[5].start, 4));
        assert_eq!(lines[0].start, prev(t, lines[5].start, 5));

        assert_eq!(lines[3].start, prev(t, lines[4].end - 1, 1));
        assert_eq!(lines[2].start, prev(t, lines[4].start, 1));

        assert_eq!(lines[2].start, prev(t, lines[3].end - 1, 1));
        assert_eq!(lines[1].start, prev(t, lines[3].start, 1));

        assert_eq!(lines[1].start, prev(t, lines[2].end - 1, 1));
        assert_eq!(lines[0].start, prev(t, lines[2].start, 1));

        assert_eq!(lines[0].start, prev(t, lines[1].end - 1, 1));
        assert_eq!(lines[0].start, prev(t, lines[1].start, 1));

        assert_eq!(lines[0].start, prev(t, lines[0].end - 1, 1));
        assert_eq!(lines[0].start, prev(t, lines[0].start, 1));
    }

    #[test]
    fn preceding_lines_empty1() {
        let t = "\n\n\nd\ne\nf\n";
        let lines = line_ranges(t);
        assert_eq!(9, t.len());

        assert_eq!(lines[0].start, prev(t, lines[0].end, 0));
        assert_eq!(lines[0].start, prev(t, lines[0].end, 1));
        assert_eq!(lines[1].start, prev(t, lines[1].end, 0));
        assert_eq!(lines[0].start, prev(t, lines[1].end, 1));

        assert_eq!(lines[5].start, prev(t, lines[5].end, 0));
        assert_eq!(lines[4].start, prev(t, lines[5].end, 1));
        assert_eq!(lines[3].start, prev(t, lines[5].end, 2));
        assert_eq!(lines[2].start, prev(t, lines[5].end, 3));
        assert_eq!(lines[1].start, prev(t, lines[5].end, 4));
        assert_eq!(lines[0].start, prev(t, lines[5].end, 5));
        assert_eq!(lines[0].start, prev(t, lines[5].end, 6));
    }

    #[test]
    fn preceding_lines_empty2() {
        let t = "a\n\n\nd\ne\nf\n";
        let lines = line_ranges(t);
        assert_eq!(10, t.len());

        assert_eq!(lines[0].start, prev(t, lines[0].end, 0));
        assert_eq!(lines[0].start, prev(t, lines[0].end, 1));
        assert_eq!(lines[1].start, prev(t, lines[1].end, 0));
        assert_eq!(lines[0].start, prev(t, lines[1].end, 1));

        assert_eq!(lines[5].start, prev(t, lines[5].end, 0));
        assert_eq!(lines[4].start, prev(t, lines[5].end, 1));
        assert_eq!(lines[3].start, prev(t, lines[5].end, 2));
        assert_eq!(lines[2].start, prev(t, lines[5].end, 3));
        assert_eq!(lines[1].start, prev(t, lines[5].end, 4));
        assert_eq!(lines[0].start, prev(t, lines[5].end, 5));
        assert_eq!(lines[0].start, prev(t, lines[5].end, 6));
    }
}
