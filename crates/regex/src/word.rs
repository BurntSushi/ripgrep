use std::{
    collections::HashMap,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::Arc,
};

use {
    grep_matcher::{Match, Matcher, NoError},
    regex_automata::{
        meta::Regex, util::captures::Captures, util::pool::Pool, Input,
        PatternID,
    },
};

use crate::{config::ConfiguredHIR, error::Error, matcher::RegexCaptures};

type PoolFn =
    Box<dyn Fn() -> Captures + Send + Sync + UnwindSafe + RefUnwindSafe>;

/// A matcher for implementing "word match" semantics.
#[derive(Debug)]
pub(crate) struct WordMatcher {
    /// The regex which is roughly `(?:^|\W)(<original pattern>)(?:$|\W)`.
    regex: Regex,
    /// The HIR that produced the regex above. We don't keep the HIR for the
    /// `original` regex.
    ///
    /// We put this in an `Arc` because by the time it gets here, it won't
    /// change. And because cloning and dropping an `Hir` is somewhat expensive
    /// due to its deep recursive representation.
    chir: Arc<ConfiguredHIR>,
    /// The original regex supplied by the user, which we use in a fast path
    /// to try and detect matches before deferring to slower engines.
    original: Regex,
    /// A map from capture group name to capture group index.
    names: HashMap<String, usize>,
    /// A thread-safe pool of reusable buffers for finding the match offset of
    /// the inner group.
    caps: Arc<Pool<Captures, PoolFn>>,
}

impl Clone for WordMatcher {
    fn clone(&self) -> WordMatcher {
        // We implement Clone manually so that we get a fresh Pool such that it
        // can set its own thread owner. This permits each thread usings `caps`
        // to hit the fast path.
        //
        // Note that cloning a regex is "cheap" since it uses reference
        // counting internally.
        let re = self.regex.clone();
        WordMatcher {
            regex: self.regex.clone(),
            chir: Arc::clone(&self.chir),
            original: self.original.clone(),
            names: self.names.clone(),
            caps: Arc::new(Pool::new(Box::new(move || re.create_captures()))),
        }
    }
}

impl WordMatcher {
    /// Create a new matcher from the given pattern that only produces matches
    /// that are considered "words."
    ///
    /// The given options are used to construct the regular expression
    /// internally.
    pub(crate) fn new(chir: ConfiguredHIR) -> Result<WordMatcher, Error> {
        let original = chir.clone().into_anchored().to_regex()?;
        let chir = Arc::new(chir.into_word()?);
        let regex = chir.to_regex()?;
        let caps = Arc::new(Pool::new({
            let regex = regex.clone();
            Box::new(move || regex.create_captures()) as PoolFn
        }));

        let mut names = HashMap::new();
        let it = regex.group_info().pattern_names(PatternID::ZERO);
        for (i, optional_name) in it.enumerate() {
            if let Some(name) = optional_name {
                names.insert(name.to_string(), i.checked_sub(1).unwrap());
            }
        }
        Ok(WordMatcher { regex, chir, original, names, caps })
    }

    /// Return the underlying regex used to match at word boundaries.
    ///
    /// The original regex is in the capture group at index 1.
    pub(crate) fn regex(&self) -> &Regex {
        &self.regex
    }

    /// Return the underlying HIR for the regex used to match at word
    /// boundaries.
    pub(crate) fn chir(&self) -> &ConfiguredHIR {
        &self.chir
    }

    /// Attempt to do a fast confirmation of a word match that covers a subset
    /// (but hopefully a big subset) of most cases. Ok(Some(..)) is returned
    /// when a match is found. Ok(None) is returned when there is definitively
    /// no match. Err(()) is returned when this routine could not detect
    /// whether there was a match or not.
    fn fast_find(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, ()> {
        // This is a bit hairy. The whole point here is to avoid running a
        // slower regex engine to extract capture groups. Remember, our word
        // regex looks like this:
        //
        //     (^|\W)(<original regex>)(\W|$)
        //
        // What we want are the match offsets of <original regex>. So in the
        // easy/common case, the original regex will be sandwiched between
        // two codepoints that are in the \W class. So our approach here is to
        // look for a match of the overall word regexp, strip the \W ends and
        // then check whether the original regex matches what's left. If so,
        // then we are guaranteed a correct match.
        //
        // This only works though if we know that the match is sandwiched
        // between two \W codepoints. This only occurs when neither ^ nor $
        // match. This in turn only occurs when the match is at either the
        // beginning or end of the haystack. In either of those cases, we
        // declare defeat and defer to the slower implementation.
        //
        // The reason why we cannot handle the ^/$ cases here is because we
        // can't assume anything about the original pattern. (Try commenting
        // out the checks for ^/$ below and run the tests to see examples.)
        //
        // NOTE(2023-07-31): After fixing #2574, this logic honestly still
        // doesn't seem correct. Regex composition is hard.
        let input = Input::new(haystack).span(at..haystack.len());
        let mut cand = match self.regex.find(input) {
            None => return Ok(None),
            Some(m) => Match::new(m.start(), m.end()),
        };
        if cand.start() == 0 || cand.end() == haystack.len() {
            return Err(());
        }
        // We decode the chars on either side of the match. If either char is
        // a word character, then that means the ^/$ matched and not \W. In
        // that case, we defer to the slower engine.
        let (ch, slen) = bstr::decode_utf8(&haystack[cand]);
        if ch.map_or(true, regex_syntax::is_word_character) {
            return Err(());
        }
        let (ch, elen) = bstr::decode_last_utf8(&haystack[cand]);
        if ch.map_or(true, regex_syntax::is_word_character) {
            return Err(());
        }
        let new_start = cand.start() + slen;
        let new_end = cand.end() - elen;
        // This occurs the original regex can match the empty string. In this
        // case, just bail instead of trying to get it right here since it's
        // likely a pathological case.
        if new_start > new_end {
            return Err(());
        }
        cand = cand.with_start(new_start).with_end(new_end);
        if self.original.is_match(&haystack[cand]) {
            Ok(Some(cand))
        } else {
            Err(())
        }
    }
}

impl Matcher for WordMatcher {
    type Captures = RegexCaptures;
    type Error = NoError;

    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, NoError> {
        // To make this easy to get right, we extract captures here instead of
        // calling `find_at`. The actual match is at capture group `1` instead
        // of `0`. We *could* use `find_at` here and then trim the match after
        // the fact, but that's a bit harder to get right, and it's not clear
        // if it's worth it.
        //
        // OK, well, it turns out that it is worth it! But it is quite tricky.
        // See `fast_find` for details. Effectively, this lets us skip running
        // a slower regex engine to extract capture groups in the vast majority
        // of cases. However, the slower engine is I believe required for full
        // correctness.
        match self.fast_find(haystack, at) {
            Ok(Some(m)) => return Ok(Some(m)),
            Ok(None) => return Ok(None),
            Err(()) => {}
        }

        let input = Input::new(haystack).span(at..haystack.len());
        let mut caps = self.caps.get();
        self.regex.search_captures(&input, &mut caps);
        Ok(caps.get_group(1).map(|sp| Match::new(sp.start, sp.end)))
    }

    fn new_captures(&self) -> Result<RegexCaptures, NoError> {
        Ok(RegexCaptures::with_offset(self.regex.create_captures(), 1))
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
        let input = Input::new(haystack).span(at..haystack.len());
        let caps = caps.captures_mut();
        self.regex.search_captures(&input, caps);
        Ok(caps.is_match())
    }

    // We specifically do not implement other methods like find_iter or
    // captures_iter. Namely, the iter methods are guaranteed to be correct
    // by virtue of implementing find_at and captures_at above.
}

#[cfg(test)]
mod tests {
    use super::WordMatcher;
    use crate::config::Config;
    use grep_matcher::{Captures, Match, Matcher};

    fn matcher(pattern: &str) -> WordMatcher {
        let chir = Config::default().build_many(&[pattern]).unwrap();
        WordMatcher::new(chir).unwrap()
    }

    fn find(pattern: &str, haystack: &str) -> Option<(usize, usize)> {
        matcher(pattern)
            .find(haystack.as_bytes())
            .unwrap()
            .map(|m| (m.start(), m.end()))
    }

    fn find_by_caps(pattern: &str, haystack: &str) -> Option<(usize, usize)> {
        let m = matcher(pattern);
        let mut caps = m.new_captures().unwrap();
        if !m.captures(haystack.as_bytes(), &mut caps).unwrap() {
            None
        } else {
            caps.get(0).map(|m| (m.start(), m.end()))
        }
    }

    // Test that the standard `find` API reports offsets correctly.
    #[test]
    fn various_find() {
        assert_eq!(Some((0, 3)), find(r"foo", "foo"));
        assert_eq!(Some((0, 3)), find(r"foo", "foo("));
        assert_eq!(Some((1, 4)), find(r"foo", "!foo("));
        assert_eq!(None, find(r"foo", "!afoo("));

        assert_eq!(Some((0, 3)), find(r"foo", "foo☃"));
        assert_eq!(None, find(r"foo", "fooб"));

        assert_eq!(Some((0, 4)), find(r"foo5", "foo5"));
        assert_eq!(None, find(r"foo", "foo5"));

        assert_eq!(Some((1, 4)), find(r"foo", "!foo!"));
        assert_eq!(Some((1, 5)), find(r"foo!", "!foo!"));
        assert_eq!(Some((0, 5)), find(r"!foo!", "!foo!"));

        assert_eq!(Some((0, 3)), find(r"foo", "foo\n"));
        assert_eq!(Some((1, 4)), find(r"foo", "!foo!\n"));
        assert_eq!(Some((1, 5)), find(r"foo!", "!foo!\n"));
        assert_eq!(Some((0, 5)), find(r"!foo!", "!foo!\n"));

        assert_eq!(Some((1, 6)), find(r"!?foo!?", "!!foo!!"));
        assert_eq!(Some((0, 5)), find(r"!?foo!?", "!foo!"));
        assert_eq!(Some((2, 5)), find(r"!?foo!?", "a!foo!a"));

        assert_eq!(Some((2, 7)), find(r"!?foo!?", "##!foo!\n"));
        assert_eq!(Some((3, 8)), find(r"!?foo!?", "##\n!foo!##"));
        assert_eq!(Some((3, 8)), find(r"!?foo!?", "##\n!foo!\n##"));
        assert_eq!(Some((3, 7)), find(r"f?oo!?", "##\nfoo!##"));
        assert_eq!(Some((2, 5)), find(r"(?-u)foo[^a]*", "#!foo☃aaa"));
    }

    // See: https://github.com/BurntSushi/ripgrep/issues/389
    #[test]
    fn regression_dash() {
        assert_eq!(Some((0, 2)), find(r"-2", "-2"));
    }

    // Test that the captures API also reports offsets correctly, just as
    // find does. This exercises a different path in the code since captures
    // are handled differently.
    #[test]
    fn various_captures() {
        assert_eq!(Some((0, 3)), find_by_caps(r"foo", "foo"));
        assert_eq!(Some((0, 3)), find_by_caps(r"foo", "foo("));
        assert_eq!(Some((1, 4)), find_by_caps(r"foo", "!foo("));
        assert_eq!(None, find_by_caps(r"foo", "!afoo("));

        assert_eq!(Some((0, 3)), find_by_caps(r"foo", "foo☃"));
        assert_eq!(None, find_by_caps(r"foo", "fooб"));
        // assert_eq!(Some((0, 3)), find_by_caps(r"foo", "fooб"));

        // See: https://github.com/BurntSushi/ripgrep/issues/389
        assert_eq!(Some((0, 2)), find_by_caps(r"-2", "-2"));
    }

    // Test that the capture reporting methods work as advertised.
    #[test]
    fn capture_indexing() {
        let m = matcher(r"(a)(?P<foo>b)(c)");
        assert_eq!(4, m.capture_count());
        assert_eq!(Some(2), m.capture_index("foo"));

        let mut caps = m.new_captures().unwrap();
        assert_eq!(4, caps.len());

        assert!(m.captures(b"abc", &mut caps).unwrap());
        assert_eq!(caps.get(0), Some(Match::new(0, 3)));
        assert_eq!(caps.get(1), Some(Match::new(0, 1)));
        assert_eq!(caps.get(2), Some(Match::new(1, 2)));
        assert_eq!(caps.get(3), Some(Match::new(2, 3)));
        assert_eq!(caps.get(4), None);

        assert!(m.captures(b"#abc#", &mut caps).unwrap());
        assert_eq!(caps.get(0), Some(Match::new(1, 4)));
        assert_eq!(caps.get(1), Some(Match::new(1, 2)));
        assert_eq!(caps.get(2), Some(Match::new(2, 3)));
        assert_eq!(caps.get(3), Some(Match::new(3, 4)));
        assert_eq!(caps.get(4), None);
    }
}
