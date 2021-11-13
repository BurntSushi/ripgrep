use grep_matcher::{Captures, Match, Matcher};
use regex::bytes::Regex;

use crate::util::{RegexMatcher, RegexMatcherNoCaps};

fn matcher(pattern: &str) -> RegexMatcher {
    RegexMatcher::new(Regex::new(pattern).unwrap())
}

fn matcher_no_caps(pattern: &str) -> RegexMatcherNoCaps {
    RegexMatcherNoCaps(Regex::new(pattern).unwrap())
}

fn m(start: usize, end: usize) -> Match {
    Match::new(start, end)
}

#[test]
fn find() {
    let matcher = matcher(r"(\w+)\s+(\w+)");
    assert_eq!(matcher.find(b" homer simpson ").unwrap(), Some(m(1, 14)));
}

#[test]
fn find_iter() {
    let matcher = matcher(r"(\w+)\s+(\w+)");
    let mut matches = vec![];
    matcher
        .find_iter(b"aa bb cc dd", |m| {
            matches.push(m);
            true
        })
        .unwrap();
    assert_eq!(matches, vec![m(0, 5), m(6, 11)]);

    // Test that find_iter respects short circuiting.
    matches.clear();
    matcher
        .find_iter(b"aa bb cc dd", |m| {
            matches.push(m);
            false
        })
        .unwrap();
    assert_eq!(matches, vec![m(0, 5)]);
}

#[test]
fn try_find_iter() {
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct MyError;

    let matcher = matcher(r"(\w+)\s+(\w+)");
    let mut matches = vec![];
    let err = matcher
        .try_find_iter(b"aa bb cc dd", |m| {
            if matches.is_empty() {
                matches.push(m);
                Ok(true)
            } else {
                Err(MyError)
            }
        })
        .unwrap()
        .unwrap_err();
    assert_eq!(matches, vec![m(0, 5)]);
    assert_eq!(err, MyError);
}

#[test]
fn shortest_match() {
    let matcher = matcher(r"a+");
    // This tests that the default impl isn't doing anything smart, and simply
    // defers to `find`.
    assert_eq!(matcher.shortest_match(b"aaa").unwrap(), Some(3));
    // The actual underlying regex is smarter.
    assert_eq!(matcher.re.shortest_match(b"aaa"), Some(1));
}

#[test]
fn captures() {
    let matcher = matcher(r"(?P<a>\w+)\s+(?P<b>\w+)");
    assert_eq!(matcher.capture_count(), 3);
    assert_eq!(matcher.capture_index("a"), Some(1));
    assert_eq!(matcher.capture_index("b"), Some(2));
    assert_eq!(matcher.capture_index("nada"), None);

    let mut caps = matcher.new_captures().unwrap();
    assert!(matcher.captures(b" homer simpson ", &mut caps).unwrap());
    assert_eq!(caps.get(0), Some(m(1, 14)));
    assert_eq!(caps.get(1), Some(m(1, 6)));
    assert_eq!(caps.get(2), Some(m(7, 14)));
}

#[test]
fn captures_iter() {
    let matcher = matcher(r"(?P<a>\w+)\s+(?P<b>\w+)");
    let mut caps = matcher.new_captures().unwrap();
    let mut matches = vec![];
    matcher
        .captures_iter(b"aa bb cc dd", &mut caps, |caps| {
            matches.push(caps.get(0).unwrap());
            matches.push(caps.get(1).unwrap());
            matches.push(caps.get(2).unwrap());
            true
        })
        .unwrap();
    assert_eq!(
        matches,
        vec![m(0, 5), m(0, 2), m(3, 5), m(6, 11), m(6, 8), m(9, 11),]
    );

    // Test that captures_iter respects short circuiting.
    matches.clear();
    matcher
        .captures_iter(b"aa bb cc dd", &mut caps, |caps| {
            matches.push(caps.get(0).unwrap());
            matches.push(caps.get(1).unwrap());
            matches.push(caps.get(2).unwrap());
            false
        })
        .unwrap();
    assert_eq!(matches, vec![m(0, 5), m(0, 2), m(3, 5),]);
}

#[test]
fn try_captures_iter() {
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct MyError;

    let matcher = matcher(r"(?P<a>\w+)\s+(?P<b>\w+)");
    let mut caps = matcher.new_captures().unwrap();
    let mut matches = vec![];
    let err = matcher
        .try_captures_iter(b"aa bb cc dd", &mut caps, |caps| {
            if matches.is_empty() {
                matches.push(caps.get(0).unwrap());
                matches.push(caps.get(1).unwrap());
                matches.push(caps.get(2).unwrap());
                Ok(true)
            } else {
                Err(MyError)
            }
        })
        .unwrap()
        .unwrap_err();
    assert_eq!(matches, vec![m(0, 5), m(0, 2), m(3, 5)]);
    assert_eq!(err, MyError);
}

// Test that our default impls for capturing are correct. Namely, when
// capturing isn't supported by the underlying matcher, then all of the
// various capturing related APIs fail fast.
#[test]
fn no_captures() {
    let matcher = matcher_no_caps(r"(?P<a>\w+)\s+(?P<b>\w+)");
    assert_eq!(matcher.capture_count(), 0);
    assert_eq!(matcher.capture_index("a"), None);
    assert_eq!(matcher.capture_index("b"), None);
    assert_eq!(matcher.capture_index("nada"), None);

    let mut caps = matcher.new_captures().unwrap();
    assert!(!matcher.captures(b"homer simpson", &mut caps).unwrap());

    let mut called = false;
    matcher
        .captures_iter(b"homer simpson", &mut caps, |_| {
            called = true;
            true
        })
        .unwrap();
    assert!(!called);
}

#[test]
fn replace() {
    let matcher = matcher(r"(\w+)\s+(\w+)");
    let mut dst = vec![];
    matcher
        .replace(b"aa bb cc dd", &mut dst, |_, dst| {
            dst.push(b'z');
            true
        })
        .unwrap();
    assert_eq!(dst, b"z z");

    // Test that replacements respect short circuiting.
    dst.clear();
    matcher
        .replace(b"aa bb cc dd", &mut dst, |_, dst| {
            dst.push(b'z');
            false
        })
        .unwrap();
    assert_eq!(dst, b"z cc dd");
}

#[test]
fn replace_with_captures() {
    let matcher = matcher(r"(\w+)\s+(\w+)");
    let haystack = b"aa bb cc dd";
    let mut caps = matcher.new_captures().unwrap();
    let mut dst = vec![];
    matcher
        .replace_with_captures(haystack, &mut caps, &mut dst, |caps, dst| {
            caps.interpolate(
                |name| matcher.capture_index(name),
                haystack,
                b"$2 $1",
                dst,
            );
            true
        })
        .unwrap();
    assert_eq!(dst, b"bb aa dd cc");

    // Test that replacements respect short circuiting.
    dst.clear();
    matcher
        .replace_with_captures(haystack, &mut caps, &mut dst, |caps, dst| {
            caps.interpolate(
                |name| matcher.capture_index(name),
                haystack,
                b"$2 $1",
                dst,
            );
            false
        })
        .unwrap();
    assert_eq!(dst, b"bb aa cc dd");
}
