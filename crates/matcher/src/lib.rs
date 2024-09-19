/*!
This crate provides an interface for regular expressions, with a focus on line
oriented search. The purpose of this crate is to provide a low level matching
interface that permits any kind of substring or regex implementation to power
the search routines provided by the
[`grep-searcher`](https://docs.rs/grep-searcher)
crate.

The primary thing provided by this crate is the [`Matcher`] trait. The trait
defines an abstract interface for text search. It is robust enough to support
everything from basic substring search all the way to arbitrarily complex
regular expression implementations without sacrificing performance.

A key design decision made in this crate is the use of *internal iteration*,
or otherwise known as the "push" model of searching. In this paradigm,
implementations of the `Matcher` trait will drive search and execute callbacks
provided by the caller when a match is found. This is in contrast to the
usual style of *external iteration* (the "pull" model) found throughout the
Rust ecosystem. There are two primary reasons why internal iteration was
chosen:

* Some search implementations may themselves require internal iteration.
  Converting an internal iterator to an external iterator can be non-trivial
  and sometimes even practically impossible.
* Rust's type system isn't quite expressive enough to write a generic interface
  using external iteration without giving something else up (namely, ease of
  use and/or performance).

In other words, internal iteration was chosen because it is the lowest common
denominator and because it is probably the least bad way of expressing the
interface in today's Rust. As a result, this trait isn't specifically intended
for everyday use, although, you might find it to be a happy price to pay if you
want to write code that is generic over multiple different regex
implementations.
*/

#![deny(missing_docs)]

use crate::interpolate::interpolate;

mod interpolate;

/// The type of a match.
///
/// The type of a match is a possibly empty range pointing to a contiguous
/// block of addressable memory.
///
/// Every `Match` is guaranteed to satisfy the invariant that `start <= end`.
///
/// # Indexing
///
/// This type is structurally identical to `std::ops::Range<usize>`, but
/// is a bit more ergonomic for dealing with match indices. In particular,
/// this type implements `Copy` and provides methods for building new `Match`
/// values based on old `Match` values. Finally, the invariant that `start`
/// is always less than or equal to `end` is enforced.
///
/// A `Match` can be used to slice a `&[u8]`, `&mut [u8]` or `&str` using
/// range notation. e.g.,
///
/// ```
/// use grep_matcher::Match;
///
/// let m = Match::new(2, 5);
/// let bytes = b"abcdefghi";
/// assert_eq!(b"cde", &bytes[m]);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Match {
    start: usize,
    end: usize,
}

impl Match {
    /// Create a new match.
    ///
    /// # Panics
    ///
    /// This function panics if `start > end`.
    #[inline]
    pub fn new(start: usize, end: usize) -> Match {
        assert!(start <= end);
        Match { start, end }
    }

    /// Creates a zero width match at the given offset.
    #[inline]
    pub fn zero(offset: usize) -> Match {
        Match { start: offset, end: offset }
    }

    /// Return the start offset of this match.
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Return the end offset of this match.
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    /// Return a new match with the start offset replaced with the given
    /// value.
    ///
    /// # Panics
    ///
    /// This method panics if `start > self.end`.
    #[inline]
    pub fn with_start(&self, start: usize) -> Match {
        assert!(start <= self.end, "{} is not <= {}", start, self.end);
        Match { start, ..*self }
    }

    /// Return a new match with the end offset replaced with the given
    /// value.
    ///
    /// # Panics
    ///
    /// This method panics if `self.start > end`.
    #[inline]
    pub fn with_end(&self, end: usize) -> Match {
        assert!(self.start <= end, "{} is not <= {}", self.start, end);
        Match { end, ..*self }
    }

    /// Offset this match by the given amount and return a new match.
    ///
    /// This adds the given offset to the start and end of this match, and
    /// returns the resulting match.
    ///
    /// # Panics
    ///
    /// This panics if adding the given amount to either the start or end
    /// offset would result in an overflow.
    #[inline]
    pub fn offset(&self, amount: usize) -> Match {
        Match {
            start: self.start.checked_add(amount).unwrap(),
            end: self.end.checked_add(amount).unwrap(),
        }
    }

    /// Returns the number of bytes in this match.
    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns true if and only if this match is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::ops::Index<Match> for [u8] {
    type Output = [u8];

    #[inline]
    fn index(&self, index: Match) -> &[u8] {
        &self[index.start..index.end]
    }
}

impl std::ops::IndexMut<Match> for [u8] {
    #[inline]
    fn index_mut(&mut self, index: Match) -> &mut [u8] {
        &mut self[index.start..index.end]
    }
}

impl std::ops::Index<Match> for str {
    type Output = str;

    #[inline]
    fn index(&self, index: Match) -> &str {
        &self[index.start..index.end]
    }
}

/// A line terminator.
///
/// A line terminator represents the end of a line. Generally, every line is
/// either "terminated" by the end of a stream or a specific byte (or sequence
/// of bytes).
///
/// Generally, a line terminator is a single byte, specifically, `\n`, on
/// Unix-like systems. On Windows, a line terminator is `\r\n` (referred to
/// as `CRLF` for `Carriage Return; Line Feed`).
///
/// The default line terminator is `\n` on all platforms.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LineTerminator(LineTerminatorImp);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum LineTerminatorImp {
    /// Any single byte representing a line terminator.
    Byte(u8),
    /// A line terminator represented by `\r\n`.
    ///
    /// When this option is used, consumers may generally treat a lone `\n` as
    /// a line terminator in addition to `\r\n`.
    CRLF,
}

impl LineTerminator {
    /// Return a new single-byte line terminator. Any byte is valid.
    #[inline]
    pub fn byte(byte: u8) -> LineTerminator {
        LineTerminator(LineTerminatorImp::Byte(byte))
    }

    /// Return a new line terminator represented by `\r\n`.
    ///
    /// When this option is used, consumers may generally treat a lone `\n` as
    /// a line terminator in addition to `\r\n`.
    #[inline]
    pub fn crlf() -> LineTerminator {
        LineTerminator(LineTerminatorImp::CRLF)
    }

    /// Returns true if and only if this line terminator is CRLF.
    #[inline]
    pub fn is_crlf(&self) -> bool {
        self.0 == LineTerminatorImp::CRLF
    }

    /// Returns this line terminator as a single byte.
    ///
    /// If the line terminator is CRLF, then this returns `\n`. This is
    /// useful for routines that, for example, find line boundaries by treating
    /// `\n` as a line terminator even when it isn't preceded by `\r`.
    #[inline]
    pub fn as_byte(&self) -> u8 {
        match self.0 {
            LineTerminatorImp::Byte(byte) => byte,
            LineTerminatorImp::CRLF => b'\n',
        }
    }

    /// Returns this line terminator as a sequence of bytes.
    ///
    /// This returns a singleton sequence for all line terminators except for
    /// `CRLF`, in which case, it returns `\r\n`.
    ///
    /// The slice returned is guaranteed to have length at least `1`.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self.0 {
            LineTerminatorImp::Byte(ref byte) => std::slice::from_ref(byte),
            LineTerminatorImp::CRLF => &[b'\r', b'\n'],
        }
    }

    /// Returns true if and only if the given slice ends with this line
    /// terminator.
    ///
    /// If this line terminator is `CRLF`, then this only checks whether the
    /// last byte is `\n`.
    #[inline]
    pub fn is_suffix(&self, slice: &[u8]) -> bool {
        slice.last().map_or(false, |&b| b == self.as_byte())
    }
}

impl Default for LineTerminator {
    #[inline]
    fn default() -> LineTerminator {
        LineTerminator::byte(b'\n')
    }
}

/// A set of bytes.
///
/// In this crate, byte sets are used to express bytes that can never appear
/// anywhere in a match for a particular implementation of the `Matcher` trait.
/// Specifically, if such a set can be determined, then it's possible for
/// callers to perform additional operations on the basis that certain bytes
/// may never match.
///
/// For example, if a search is configured to possibly produce results that
/// span multiple lines but a caller provided pattern can never match across
/// multiple lines, then it may make sense to divert to more optimized line
/// oriented routines that don't need to handle the multi-line match case.
#[derive(Clone, Debug)]
pub struct ByteSet(BitSet);

#[derive(Clone, Copy)]
struct BitSet([u64; 4]);

impl std::fmt::Debug for BitSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fmtd = f.debug_set();
        for b in 0..=255 {
            if ByteSet(*self).contains(b) {
                fmtd.entry(&b);
            }
        }
        fmtd.finish()
    }
}

impl ByteSet {
    /// Create an empty set of bytes.
    #[inline]
    pub fn empty() -> ByteSet {
        ByteSet(BitSet([0; 4]))
    }

    /// Create a full set of bytes such that every possible byte is in the set
    /// returned.
    #[inline]
    pub fn full() -> ByteSet {
        ByteSet(BitSet([u64::MAX; 4]))
    }

    /// Add a byte to this set.
    ///
    /// If the given byte already belongs to this set, then this is a no-op.
    #[inline]
    pub fn add(&mut self, byte: u8) {
        let bucket = byte / 64;
        let bit = byte % 64;
        (self.0).0[usize::from(bucket)] |= 1 << bit;
    }

    /// Add an inclusive range of bytes.
    #[inline]
    pub fn add_all(&mut self, start: u8, end: u8) {
        for b in start..=end {
            self.add(b);
        }
    }

    /// Remove a byte from this set.
    ///
    /// If the given byte is not in this set, then this is a no-op.
    #[inline]
    pub fn remove(&mut self, byte: u8) {
        let bucket = byte / 64;
        let bit = byte % 64;
        (self.0).0[usize::from(bucket)] &= !(1 << bit);
    }

    /// Remove an inclusive range of bytes.
    #[inline]
    pub fn remove_all(&mut self, start: u8, end: u8) {
        for b in start..=end {
            self.remove(b);
        }
    }

    /// Return true if and only if the given byte is in this set.
    #[inline]
    pub fn contains(&self, byte: u8) -> bool {
        let bucket = byte / 64;
        let bit = byte % 64;
        (self.0).0[usize::from(bucket)] & (1 << bit) > 0
    }
}

/// A trait that describes implementations of capturing groups.
///
/// When a matcher supports capturing group extraction, then it is the
/// matcher's responsibility to provide an implementation of this trait.
///
/// Principally, this trait provides a way to access capturing groups
/// in a uniform way that does not require any specific representation.
/// Namely, different matcher implementations may require different in-memory
/// representations of capturing groups. This trait permits matchers to
/// maintain their specific in-memory representation.
///
/// Note that this trait explicitly does not provide a way to construct a new
/// capture value. Instead, it is the responsibility of a `Matcher` to build
/// one, which might require knowledge of the matcher's internal implementation
/// details.
pub trait Captures {
    /// Return the total number of capturing groups. This includes capturing
    /// groups that have not matched anything.
    fn len(&self) -> usize;

    /// Return the capturing group match at the given index. If no match of
    /// that capturing group exists, then this returns `None`.
    ///
    /// When a matcher reports a match with capturing groups, then the first
    /// capturing group (at index `0`) must always correspond to the offsets
    /// for the overall match.
    fn get(&self, i: usize) -> Option<Match>;

    /// Return the overall match for the capture.
    ///
    /// This returns the match for index `0`. That is it is equivalent to
    /// `get(0).unwrap()`
    #[inline]
    fn as_match(&self) -> Match {
        self.get(0).unwrap()
    }

    /// Returns true if and only if these captures are empty. This occurs
    /// when `len` is `0`.
    ///
    /// Note that capturing groups that have non-zero length but otherwise
    /// contain no matching groups are *not* empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Expands all instances of `$name` in `replacement` to the corresponding
    /// capture group `name`, and writes them to the `dst` buffer given.
    ///
    /// (Note: If you're looking for a convenient way to perform replacements
    /// with interpolation, then you'll want to use the `replace_with_captures`
    /// method on the `Matcher` trait.)
    ///
    /// `name` may be an integer corresponding to the index of the
    /// capture group (counted by order of opening parenthesis where `0` is the
    /// entire match) or it can be a name (consisting of letters, digits or
    /// underscores) corresponding to a named capture group.
    ///
    /// A `name` is translated to a capture group index via the given
    /// `name_to_index` function. If `name` isn't a valid capture group
    /// (whether the name doesn't exist or isn't a valid index), then it is
    /// replaced with the empty string.
    ///
    /// The longest possible name is used. e.g., `$1a` looks up the capture
    /// group named `1a` and not the capture group at index `1`. To exert
    /// more precise control over the name, use braces, e.g., `${1}a`. In all
    /// cases, capture group names are limited to ASCII letters, numbers and
    /// underscores.
    ///
    /// To write a literal `$` use `$$`.
    ///
    /// Note that the capture group match indices are resolved by slicing
    /// the given `haystack`. Generally, this means that `haystack` should be
    /// the same slice that was searched to get the current capture group
    /// matches.
    #[inline]
    fn interpolate<F>(
        &self,
        name_to_index: F,
        haystack: &[u8],
        replacement: &[u8],
        dst: &mut Vec<u8>,
    ) where
        F: FnMut(&str) -> Option<usize>,
    {
        interpolate(
            replacement,
            |i, dst| {
                if let Some(range) = self.get(i) {
                    dst.extend(&haystack[range]);
                }
            },
            name_to_index,
            dst,
        )
    }
}

/// NoCaptures provides an always-empty implementation of the `Captures` trait.
///
/// This type is useful for implementations of `Matcher` that don't support
/// capturing groups.
#[derive(Clone, Debug)]
pub struct NoCaptures(());

impl NoCaptures {
    /// Create an empty set of capturing groups.
    #[inline]
    pub fn new() -> NoCaptures {
        NoCaptures(())
    }
}

impl Captures for NoCaptures {
    #[inline]
    fn len(&self) -> usize {
        0
    }

    #[inline]
    fn get(&self, _: usize) -> Option<Match> {
        None
    }
}

/// NoError provides an error type for matchers that never produce errors.
///
/// This error type implements the `std::error::Error` and `std::fmt::Display`
/// traits for use in matcher implementations that can never produce errors.
///
/// The `std::fmt::Debug` and `std::fmt::Display` impls for this type panics.
#[derive(Debug, Eq, PartialEq)]
pub struct NoError(());

impl std::error::Error for NoError {
    fn description(&self) -> &str {
        "no error"
    }
}

impl std::fmt::Display for NoError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("BUG for NoError: an impossible error occurred")
    }
}

impl From<NoError> for std::io::Error {
    fn from(_: NoError) -> std::io::Error {
        panic!("BUG for NoError: an impossible error occurred")
    }
}

/// The type of match for a line oriented matcher.
#[derive(Clone, Copy, Debug)]
pub enum LineMatchKind {
    /// A position inside a line that is known to contain a match.
    ///
    /// This position can be anywhere in the line. It does not need to point
    /// at the location of the match.
    Confirmed(usize),
    /// A position inside a line that may contain a match, and must be searched
    /// for verification.
    ///
    /// This position can be anywhere in the line. It does not need to point
    /// at the location of the match.
    Candidate(usize),
}

/// A matcher defines an interface for regular expression implementations.
///
/// While this trait is large, there are only two required methods that
/// implementors must provide: `find_at` and `new_captures`. If captures aren't
/// supported by your implementation, then `new_captures` can be implemented
/// with [`NoCaptures`]. If your implementation does support capture groups,
/// then you should also implement the other capture related methods, as
/// dictated by the documentation. Crucially, this includes `captures_at`.
///
/// The rest of the methods on this trait provide default implementations on
/// top of `find_at` and `new_captures`. It is not uncommon for implementations
/// to be able to provide faster variants of some methods; in those cases,
/// simply override the default implementation.
pub trait Matcher {
    /// The concrete type of capturing groups used for this matcher.
    ///
    /// If this implementation does not support capturing groups, then set
    /// this to `NoCaptures`.
    type Captures: Captures;

    /// The error type used by this matcher.
    ///
    /// For matchers in which an error is not possible, they are encouraged to
    /// use the `NoError` type in this crate. In the future, when the "never"
    /// (spelled `!`) type is stabilized, then it should probably be used
    /// instead.
    type Error: std::fmt::Display;

    /// Returns the start and end byte range of the first match in `haystack`
    /// after `at`, where the byte offsets are relative to that start of
    /// `haystack` (and not `at`). If no match exists, then `None` is returned.
    ///
    /// The text encoding of `haystack` is not strictly specified. Matchers are
    /// advised to assume UTF-8, or at worst, some ASCII compatible encoding.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, Self::Error>;

    /// Creates an empty group of captures suitable for use with the capturing
    /// APIs of this trait.
    ///
    /// Implementations that don't support capturing groups should use
    /// the `NoCaptures` type and implement this method by calling
    /// `NoCaptures::new()`.
    fn new_captures(&self) -> Result<Self::Captures, Self::Error>;

    /// Returns the total number of capturing groups in this matcher.
    ///
    /// If a matcher supports capturing groups, then this value must always be
    /// at least 1, where the first capturing group always corresponds to the
    /// overall match.
    ///
    /// If a matcher does not support capturing groups, then this should
    /// always return 0.
    ///
    /// By default, capturing groups are not supported, so this always
    /// returns 0.
    #[inline]
    fn capture_count(&self) -> usize {
        0
    }

    /// Maps the given capture group name to its corresponding capture group
    /// index, if one exists. If one does not exist, then `None` is returned.
    ///
    /// If the given capture group name maps to multiple indices, then it is
    /// not specified which one is returned. However, it is guaranteed that
    /// one of them is returned.
    ///
    /// By default, capturing groups are not supported, so this always returns
    /// `None`.
    #[inline]
    fn capture_index(&self, _name: &str) -> Option<usize> {
        None
    }

    /// Returns the start and end byte range of the first match in `haystack`.
    /// If no match exists, then `None` is returned.
    ///
    /// The text encoding of `haystack` is not strictly specified. Matchers are
    /// advised to assume UTF-8, or at worst, some ASCII compatible encoding.
    #[inline]
    fn find(&self, haystack: &[u8]) -> Result<Option<Match>, Self::Error> {
        self.find_at(haystack, 0)
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack`. If no match exists, then the given function is never
    /// called. If the function returns `false`, then iteration stops.
    #[inline]
    fn find_iter<F>(
        &self,
        haystack: &[u8],
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        self.find_iter_at(haystack, 0, matched)
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack`. If no match exists, then the given function is never
    /// called. If the function returns `false`, then iteration stops.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    #[inline]
    fn find_iter_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        mut matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        self.try_find_iter_at(haystack, at, |m| Ok(matched(m)))
            .map(|r: Result<(), ()>| r.unwrap())
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack`. If no match exists, then the given function is never
    /// called. If the function returns `false`, then iteration stops.
    /// Similarly, if the function returns an error then iteration stops and
    /// the error is yielded. If an error occurs while executing the search,
    /// then it is converted to
    /// `E`.
    #[inline]
    fn try_find_iter<F, E>(
        &self,
        haystack: &[u8],
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        self.try_find_iter_at(haystack, 0, matched)
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack`. If no match exists, then the given function is never
    /// called. If the function returns `false`, then iteration stops.
    /// Similarly, if the function returns an error then iteration stops and
    /// the error is yielded. If an error occurs while executing the search,
    /// then it is converted to
    /// `E`.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    #[inline]
    fn try_find_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        mut matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        let mut last_end = at;
        let mut last_match = None;

        loop {
            if last_end > haystack.len() {
                return Ok(Ok(()));
            }
            let m = match self.find_at(haystack, last_end)? {
                None => return Ok(Ok(())),
                Some(m) => m,
            };
            if m.start == m.end {
                // This is an empty match. To ensure we make progress, start
                // the next search at the smallest possible starting position
                // of the next match following this one.
                last_end = m.end + 1;
                // Don't accept empty matches immediately following a match.
                // Just move on to the next match.
                if Some(m.end) == last_match {
                    continue;
                }
            } else {
                last_end = m.end;
            }
            last_match = Some(m.end);
            match matched(m) {
                Ok(true) => continue,
                Ok(false) => return Ok(Ok(())),
                Err(err) => return Ok(Err(err)),
            }
        }
    }

    /// Populates the first set of capture group matches from `haystack` into
    /// `caps`. If no match exists, then `false` is returned.
    ///
    /// The text encoding of `haystack` is not strictly specified. Matchers are
    /// advised to assume UTF-8, or at worst, some ASCII compatible encoding.
    #[inline]
    fn captures(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
    ) -> Result<bool, Self::Error> {
        self.captures_at(haystack, 0, caps)
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack` with capture groups extracted from each match. If no
    /// match exists, then the given function is never called. If the function
    /// returns `false`, then iteration stops.
    #[inline]
    fn captures_iter<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        self.captures_iter_at(haystack, 0, caps, matched)
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack` with capture groups extracted from each match. If no
    /// match exists, then the given function is never called. If the function
    /// returns `false`, then iteration stops.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    #[inline]
    fn captures_iter_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        mut matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        self.try_captures_iter_at(haystack, at, caps, |caps| Ok(matched(caps)))
            .map(|r: Result<(), ()>| r.unwrap())
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack` with capture groups extracted from each match. If no
    /// match exists, then the given function is never called. If the function
    /// returns `false`, then iteration stops. Similarly, if the function
    /// returns an error then iteration stops and the error is yielded. If
    /// an error occurs while executing the search, then it is converted to
    /// `E`.
    #[inline]
    fn try_captures_iter<F, E>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        self.try_captures_iter_at(haystack, 0, caps, matched)
    }

    /// Executes the given function over successive non-overlapping matches
    /// in `haystack` with capture groups extracted from each match. If no
    /// match exists, then the given function is never called. If the function
    /// returns `false`, then iteration stops. Similarly, if the function
    /// returns an error then iteration stops and the error is yielded. If
    /// an error occurs while executing the search, then it is converted to
    /// `E`.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    #[inline]
    fn try_captures_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        mut matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        let mut last_end = at;
        let mut last_match = None;

        loop {
            if last_end > haystack.len() {
                return Ok(Ok(()));
            }
            if !self.captures_at(haystack, last_end, caps)? {
                return Ok(Ok(()));
            }
            let m = caps.get(0).unwrap();
            if m.start == m.end {
                // This is an empty match. To ensure we make progress, start
                // the next search at the smallest possible starting position
                // of the next match following this one.
                last_end = m.end + 1;
                // Don't accept empty matches immediately following a match.
                // Just move on to the next match.
                if Some(m.end) == last_match {
                    continue;
                }
            } else {
                last_end = m.end;
            }
            last_match = Some(m.end);
            match matched(caps) {
                Ok(true) => continue,
                Ok(false) => return Ok(Ok(())),
                Err(err) => return Ok(Err(err)),
            }
        }
    }

    /// Populates the first set of capture group matches from `haystack`
    /// into `matches` after `at`, where the byte offsets in each capturing
    /// group are relative to the start of `haystack` (and not `at`). If no
    /// match exists, then `false` is returned and the contents of the given
    /// capturing groups are unspecified.
    ///
    /// The text encoding of `haystack` is not strictly specified. Matchers are
    /// advised to assume UTF-8, or at worst, some ASCII compatible encoding.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    ///
    /// By default, capturing groups aren't supported, and this implementation
    /// will always behave as if a match were impossible.
    ///
    /// Implementors that provide support for capturing groups must guarantee
    /// that when a match occurs, the first capture match (at index `0`) is
    /// always set to the overall match offsets.
    ///
    /// Note that if implementors seek to support capturing groups, then they
    /// should implement this method. Other methods that match based on
    /// captures will then work automatically.
    #[inline]
    fn captures_at(
        &self,
        _haystack: &[u8],
        _at: usize,
        _caps: &mut Self::Captures,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Replaces every match in the given haystack with the result of calling
    /// `append`. `append` is given the start and end of a match, along with
    /// a handle to the `dst` buffer provided.
    ///
    /// If the given `append` function returns `false`, then replacement stops.
    #[inline]
    fn replace<F>(
        &self,
        haystack: &[u8],
        dst: &mut Vec<u8>,
        mut append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Match, &mut Vec<u8>) -> bool,
    {
        let mut last_match = 0;
        self.find_iter(haystack, |m| {
            dst.extend(&haystack[last_match..m.start]);
            last_match = m.end;
            append(m, dst)
        })?;
        dst.extend(&haystack[last_match..]);
        Ok(())
    }

    /// Replaces every match in the given haystack with the result of calling
    /// `append` with the matching capture groups.
    ///
    /// If the given `append` function returns `false`, then replacement stops.
    #[inline]
    fn replace_with_captures<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        self.replace_with_captures_at(haystack, 0, caps, dst, append)
    }

    /// Replaces every match in the given haystack with the result of calling
    /// `append` with the matching capture groups.
    ///
    /// If the given `append` function returns `false`, then replacement stops.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    #[inline]
    fn replace_with_captures_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        mut append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        let mut last_match = at;
        self.captures_iter_at(haystack, at, caps, |caps| {
            let m = caps.get(0).unwrap();
            dst.extend(&haystack[last_match..m.start]);
            last_match = m.end;
            append(caps, dst)
        })?;
        dst.extend(&haystack[last_match..]);
        Ok(())
    }

    /// Returns true if and only if the matcher matches the given haystack.
    ///
    /// By default, this method is implemented by calling `shortest_match`.
    #[inline]
    fn is_match(&self, haystack: &[u8]) -> Result<bool, Self::Error> {
        self.is_match_at(haystack, 0)
    }

    /// Returns true if and only if the matcher matches the given haystack
    /// starting at the given position.
    ///
    /// By default, this method is implemented by calling `shortest_match_at`.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    #[inline]
    fn is_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<bool, Self::Error> {
        Ok(self.shortest_match_at(haystack, at)?.is_some())
    }

    /// Returns an end location of the first match in `haystack`. If no match
    /// exists, then `None` is returned.
    ///
    /// Note that the end location reported by this method may be less than the
    /// same end location reported by `find`. For example, running `find` with
    /// the pattern `a+` on the haystack `aaa` should report a range of `[0,
    /// 3)`, but `shortest_match` may report `1` as the ending location since
    /// that is the place at which a match is guaranteed to occur.
    ///
    /// This method should never report false positives or false negatives. The
    /// point of this method is that some implementors may be able to provide
    /// a faster implementation of this than what `find` does.
    ///
    /// By default, this method is implemented by calling `find`.
    #[inline]
    fn shortest_match(
        &self,
        haystack: &[u8],
    ) -> Result<Option<usize>, Self::Error> {
        self.shortest_match_at(haystack, 0)
    }

    /// Returns an end location of the first match in `haystack` starting at
    /// the given position. If no match exists, then `None` is returned.
    ///
    /// Note that the end location reported by this method may be less than the
    /// same end location reported by `find`. For example, running `find` with
    /// the pattern `a+` on the haystack `aaa` should report a range of `[0,
    /// 3)`, but `shortest_match` may report `1` as the ending location since
    /// that is the place at which a match is guaranteed to occur.
    ///
    /// This method should never report false positives or false negatives. The
    /// point of this method is that some implementors may be able to provide
    /// a faster implementation of this than what `find` does.
    ///
    /// By default, this method is implemented by calling `find_at`.
    ///
    /// The significance of the starting point is that it takes the surrounding
    /// context into consideration. For example, the `\A` anchor can only
    /// match when `at == 0`.
    #[inline]
    fn shortest_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<usize>, Self::Error> {
        Ok(self.find_at(haystack, at)?.map(|m| m.end))
    }

    /// If available, return a set of bytes that will never appear in a match
    /// produced by an implementation.
    ///
    /// Specifically, if such a set can be determined, then it's possible for
    /// callers to perform additional operations on the basis that certain
    /// bytes may never match.
    ///
    /// For example, if a search is configured to possibly produce results
    /// that span multiple lines but a caller provided pattern can never
    /// match across multiple lines, then it may make sense to divert to
    /// more optimized line oriented routines that don't need to handle the
    /// multi-line match case.
    ///
    /// Implementations that produce this set must never report false
    /// positives, but may produce false negatives. That is, is a byte is in
    /// this set then it must be guaranteed that it is never in a match. But,
    /// if a byte is not in this set, then callers cannot assume that a match
    /// exists with that byte.
    ///
    /// By default, this returns `None`.
    #[inline]
    fn non_matching_bytes(&self) -> Option<&ByteSet> {
        None
    }

    /// If this matcher was compiled as a line oriented matcher, then this
    /// method returns the line terminator if and only if the line terminator
    /// never appears in any match produced by this matcher. If this wasn't
    /// compiled as a line oriented matcher, or if the aforementioned guarantee
    /// cannot be made, then this must return `None`, which is the default.
    /// It is **never wrong** to return `None`, but returning a line terminator
    /// when it can appear in a match results in unspecified behavior.
    ///
    /// The line terminator is typically `b'\n'`, but can be any single byte or
    /// `CRLF`.
    ///
    /// By default, this returns `None`.
    #[inline]
    fn line_terminator(&self) -> Option<LineTerminator> {
        None
    }

    /// Return one of the following: a confirmed line match, a candidate line
    /// match (which may be a false positive) or no match at all (which **must
    /// not** be a false negative). When reporting a confirmed or candidate
    /// match, the position returned can be any position in the line.
    ///
    /// By default, this never returns a candidate match, and always either
    /// returns a confirmed match or no match at all.
    ///
    /// When a matcher can match spans over multiple lines, then the behavior
    /// of this method is unspecified. Namely, use of this method only
    /// makes sense in a context where the caller is looking for the next
    /// matching line. That is, callers should only use this method when
    /// `line_terminator` does not return `None`.
    ///
    /// # Design rationale
    ///
    /// A line matcher is, fundamentally, a normal matcher with the addition
    /// of one optional method: finding a line. By default, this routine
    /// is implemented via the matcher's `shortest_match` method, which
    /// always yields either no match or a `LineMatchKind::Confirmed`. However,
    /// implementors may provide a routine for this that can return candidate
    /// lines that need subsequent verification to be confirmed as a match.
    /// This can be useful in cases where it may be quicker to find candidate
    /// lines via some other means instead of relying on the more general
    /// implementations for `find` and `shortest_match`.
    ///
    /// For example, consider the regex `\w+foo\s+`. Both `find` and
    /// `shortest_match` must consider the entire regex, including the `\w+`
    /// and `\s+`, while searching. However, this method could look for lines
    /// containing `foo` and return them as candidates. Finding `foo` might
    /// be implemented as a highly optimized substring search routine (like
    /// `memmem`), which is likely to be faster than whatever more generalized
    /// routine is required for resolving `\w+foo\s+`. The caller is then
    /// responsible for confirming whether a match exists or not.
    ///
    /// Note that while this method may report false positives, it must never
    /// report false negatives. That is, it can never skip over lines that
    /// contain a match.
    #[inline]
    fn find_candidate_line(
        &self,
        haystack: &[u8],
    ) -> Result<Option<LineMatchKind>, Self::Error> {
        Ok(self.shortest_match(haystack)?.map(LineMatchKind::Confirmed))
    }
}

impl<'a, M: Matcher> Matcher for &'a M {
    type Captures = M::Captures;
    type Error = M::Error;

    #[inline]
    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, Self::Error> {
        (*self).find_at(haystack, at)
    }

    #[inline]
    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        (*self).new_captures()
    }

    #[inline]
    fn captures_at(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
    ) -> Result<bool, Self::Error> {
        (*self).captures_at(haystack, at, caps)
    }

    #[inline]
    fn capture_index(&self, name: &str) -> Option<usize> {
        (*self).capture_index(name)
    }

    #[inline]
    fn capture_count(&self) -> usize {
        (*self).capture_count()
    }

    #[inline]
    fn find(&self, haystack: &[u8]) -> Result<Option<Match>, Self::Error> {
        (*self).find(haystack)
    }

    #[inline]
    fn find_iter<F>(
        &self,
        haystack: &[u8],
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        (*self).find_iter(haystack, matched)
    }

    #[inline]
    fn find_iter_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Match) -> bool,
    {
        (*self).find_iter_at(haystack, at, matched)
    }

    #[inline]
    fn try_find_iter<F, E>(
        &self,
        haystack: &[u8],
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        (*self).try_find_iter(haystack, matched)
    }

    #[inline]
    fn try_find_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Match) -> Result<bool, E>,
    {
        (*self).try_find_iter_at(haystack, at, matched)
    }

    #[inline]
    fn captures(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
    ) -> Result<bool, Self::Error> {
        (*self).captures(haystack, caps)
    }

    #[inline]
    fn captures_iter<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        (*self).captures_iter(haystack, caps, matched)
    }

    #[inline]
    fn captures_iter_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures) -> bool,
    {
        (*self).captures_iter_at(haystack, at, caps, matched)
    }

    #[inline]
    fn try_captures_iter<F, E>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        (*self).try_captures_iter(haystack, caps, matched)
    }

    #[inline]
    fn try_captures_iter_at<F, E>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(&Self::Captures) -> Result<bool, E>,
    {
        (*self).try_captures_iter_at(haystack, at, caps, matched)
    }

    #[inline]
    fn replace<F>(
        &self,
        haystack: &[u8],
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(Match, &mut Vec<u8>) -> bool,
    {
        (*self).replace(haystack, dst, append)
    }

    #[inline]
    fn replace_with_captures<F>(
        &self,
        haystack: &[u8],
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        (*self).replace_with_captures(haystack, caps, dst, append)
    }

    #[inline]
    fn replace_with_captures_at<F>(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut Self::Captures,
        dst: &mut Vec<u8>,
        append: F,
    ) -> Result<(), Self::Error>
    where
        F: FnMut(&Self::Captures, &mut Vec<u8>) -> bool,
    {
        (*self).replace_with_captures_at(haystack, at, caps, dst, append)
    }

    #[inline]
    fn is_match(&self, haystack: &[u8]) -> Result<bool, Self::Error> {
        (*self).is_match(haystack)
    }

    #[inline]
    fn is_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<bool, Self::Error> {
        (*self).is_match_at(haystack, at)
    }

    #[inline]
    fn shortest_match(
        &self,
        haystack: &[u8],
    ) -> Result<Option<usize>, Self::Error> {
        (*self).shortest_match(haystack)
    }

    #[inline]
    fn shortest_match_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<usize>, Self::Error> {
        (*self).shortest_match_at(haystack, at)
    }

    #[inline]
    fn non_matching_bytes(&self) -> Option<&ByteSet> {
        (*self).non_matching_bytes()
    }

    #[inline]
    fn line_terminator(&self) -> Option<LineTerminator> {
        (*self).line_terminator()
    }

    #[inline]
    fn find_candidate_line(
        &self,
        haystack: &[u8],
    ) -> Result<Option<LineMatchKind>, Self::Error> {
        (*self).find_candidate_line(haystack)
    }
}
