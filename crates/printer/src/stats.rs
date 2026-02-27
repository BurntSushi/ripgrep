use std::{
    ops::{Add, AddAssign},
    time::Duration,
};

use crate::util::NiceDuration;

/// Summary statistics produced at the end of a search.
///
/// When statistics are reported by a printer, they correspond to all searches
/// executed with that printer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    elapsed: NiceDuration,
    searches: u64,
    searches_with_match: u64,
    bytes_searched: u64,
    bytes_printed: u64,
    matched_lines: u64,
    matches: u64,
}

impl Stats {
    /// Return a new value for tracking aggregate statistics across searches.
    ///
    /// All statistics are set to `0`.
    pub fn new() -> Stats {
        Stats::default()
    }

    /// Return the total amount of time elapsed.
    pub fn elapsed(&self) -> Duration {
        self.elapsed.0
    }

    /// Return the total number of searches executed.
    pub fn searches(&self) -> u64 {
        self.searches
    }

    /// Return the total number of searches that found at least one match.
    pub fn searches_with_match(&self) -> u64 {
        self.searches_with_match
    }

    /// Return the total number of bytes searched.
    pub fn bytes_searched(&self) -> u64 {
        self.bytes_searched
    }

    /// Return the total number of bytes printed.
    pub fn bytes_printed(&self) -> u64 {
        self.bytes_printed
    }

    /// Return the total number of lines that participated in a match.
    ///
    /// When matches may contain multiple lines then this includes every line
    /// that is part of every match.
    pub fn matched_lines(&self) -> u64 {
        self.matched_lines
    }

    /// Return the total number of matches.
    ///
    /// There may be multiple matches per line.
    pub fn matches(&self) -> u64 {
        self.matches
    }

    /// Add to the elapsed time.
    pub fn add_elapsed(&mut self, duration: Duration) {
        self.elapsed.0 += duration;
    }

    /// Add to the number of searches executed.
    pub fn add_searches(&mut self, n: u64) {
        self.searches += n;
    }

    /// Add to the number of searches that found at least one match.
    pub fn add_searches_with_match(&mut self, n: u64) {
        self.searches_with_match += n;
    }

    /// Add to the total number of bytes searched.
    pub fn add_bytes_searched(&mut self, n: u64) {
        self.bytes_searched += n;
    }

    /// Add to the total number of bytes printed.
    pub fn add_bytes_printed(&mut self, n: u64) {
        self.bytes_printed += n;
    }

    /// Add to the total number of lines that participated in a match.
    pub fn add_matched_lines(&mut self, n: u64) {
        self.matched_lines += n;
    }

    /// Add to the total number of matches.
    pub fn add_matches(&mut self, n: u64) {
        self.matches += n;
    }
}

impl Add for Stats {
    type Output = Stats;

    fn add(self, rhs: Stats) -> Stats {
        self + &rhs
    }
}

impl<'a> Add<&'a Stats> for Stats {
    type Output = Stats;

    fn add(self, rhs: &'a Stats) -> Stats {
        Stats {
            elapsed: NiceDuration(self.elapsed.0 + rhs.elapsed.0),
            searches: self.searches + rhs.searches,
            searches_with_match: self.searches_with_match
                + rhs.searches_with_match,
            bytes_searched: self.bytes_searched + rhs.bytes_searched,
            bytes_printed: self.bytes_printed + rhs.bytes_printed,
            matched_lines: self.matched_lines + rhs.matched_lines,
            matches: self.matches + rhs.matches,
        }
    }
}

impl AddAssign for Stats {
    fn add_assign(&mut self, rhs: Stats) {
        *self += &rhs;
    }
}

impl<'a> AddAssign<&'a Stats> for Stats {
    fn add_assign(&mut self, rhs: &'a Stats) {
        self.elapsed.0 += rhs.elapsed.0;
        self.searches += rhs.searches;
        self.searches_with_match += rhs.searches_with_match;
        self.bytes_searched += rhs.bytes_searched;
        self.bytes_printed += rhs.bytes_printed;
        self.matched_lines += rhs.matched_lines;
        self.matches += rhs.matches;
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Stats {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("Stats", 7)?;
        state.serialize_field("elapsed", &self.elapsed)?;
        state.serialize_field("searches", &self.searches)?;
        state.serialize_field(
            "searches_with_match",
            &self.searches_with_match,
        )?;
        state.serialize_field("bytes_searched", &self.bytes_searched)?;
        state.serialize_field("bytes_printed", &self.bytes_printed)?;
        state.serialize_field("matched_lines", &self.matched_lines)?;
        state.serialize_field("matches", &self.matches)?;
        state.end()
    }
}
