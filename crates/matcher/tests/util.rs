use std::collections::HashMap;

use {
    grep_matcher::{Captures, Match, Matcher, NoCaptures, NoError},
    regex::bytes::{CaptureLocations, Regex},
};

#[derive(Debug)]
pub(crate) struct RegexMatcher {
    pub re: Regex,
    pub names: HashMap<String, usize>,
}

impl RegexMatcher {
    pub(crate) fn new(re: Regex) -> RegexMatcher {
        let mut names = HashMap::new();
        for (i, optional_name) in re.capture_names().enumerate() {
            if let Some(name) = optional_name {
                names.insert(name.to_string(), i);
            }
        }
        RegexMatcher { re, names }
    }
}

type Result<T> = std::result::Result<T, NoError>;

impl Matcher for RegexMatcher {
    type Captures = RegexCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>> {
        Ok(self
            .re
            .find_at(haystack, at)
            .map(|m| Match::new(m.start(), m.end())))
    }

    fn new_captures(&self) -> Result<RegexCaptures> {
        Ok(RegexCaptures(self.re.capture_locations()))
    }

    fn captures_at(
        &self,
        haystack: &[u8],
        at: usize,
        caps: &mut RegexCaptures,
    ) -> Result<bool> {
        Ok(self.re.captures_read_at(&mut caps.0, haystack, at).is_some())
    }

    fn capture_count(&self) -> usize {
        self.re.captures_len()
    }

    fn capture_index(&self, name: &str) -> Option<usize> {
        self.names.get(name).map(|i| *i)
    }

    // We purposely don't implement any other methods, so that we test the
    // default impls. The "real" Regex impl for Matcher provides a few more
    // impls. e.g., Its `find_iter` impl is faster than what we can do here,
    // since the regex crate avoids synchronization overhead.
}

#[derive(Debug)]
pub(crate) struct RegexMatcherNoCaps(pub(crate) Regex);

impl Matcher for RegexMatcherNoCaps {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>> {
        Ok(self
            .0
            .find_at(haystack, at)
            .map(|m| Match::new(m.start(), m.end())))
    }

    fn new_captures(&self) -> Result<NoCaptures> {
        Ok(NoCaptures::new())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RegexCaptures(CaptureLocations);

impl Captures for RegexCaptures {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn get(&self, i: usize) -> Option<Match> {
        self.0.pos(i).map(|(s, e)| Match::new(s, e))
    }
}
