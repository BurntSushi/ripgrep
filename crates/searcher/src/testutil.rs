use std::io::{self, Write};

use {
    bstr::ByteSlice,
    grep_matcher::{
        LineMatchKind, LineTerminator, Match, Matcher, NoCaptures, NoError,
    },
    regex::bytes::{Regex, RegexBuilder},
};

use crate::{
    searcher::{BinaryDetection, Searcher, SearcherBuilder},
    sink::{Sink, SinkContext, SinkFinish, SinkMatch},
};

/// A simple regex matcher.
///
/// This supports setting the matcher's line terminator configuration directly,
/// which we use for testing purposes. That is, the caller explicitly
/// determines whether the line terminator optimization is enabled. (In reality
/// this optimization is detected automatically by inspecting and possibly
/// modifying the regex itself.)
#[derive(Clone, Debug)]
pub(crate) struct RegexMatcher {
    regex: Regex,
    line_term: Option<LineTerminator>,
    every_line_is_candidate: bool,
}

impl RegexMatcher {
    /// Create a new regex matcher.
    pub(crate) fn new(pattern: &str) -> RegexMatcher {
        let regex = RegexBuilder::new(pattern)
            .multi_line(true) // permits ^ and $ to match at \n boundaries
            .build()
            .unwrap();
        RegexMatcher { regex, line_term: None, every_line_is_candidate: false }
    }

    /// Forcefully set the line terminator of this matcher.
    ///
    /// By default, this matcher has no line terminator set.
    pub(crate) fn set_line_term(
        &mut self,
        line_term: Option<LineTerminator>,
    ) -> &mut RegexMatcher {
        self.line_term = line_term;
        self
    }

    /// Whether to return every line as a candidate or not.
    ///
    /// This forces searchers to handle the case of reporting a false positive.
    pub(crate) fn every_line_is_candidate(
        &mut self,
        yes: bool,
    ) -> &mut RegexMatcher {
        self.every_line_is_candidate = yes;
        self
    }
}

impl Matcher for RegexMatcher {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(
        &self,
        haystack: &[u8],
        at: usize,
    ) -> Result<Option<Match>, NoError> {
        Ok(self
            .regex
            .find_at(haystack, at)
            .map(|m| Match::new(m.start(), m.end())))
    }

    fn new_captures(&self) -> Result<NoCaptures, NoError> {
        Ok(NoCaptures::new())
    }

    fn line_terminator(&self) -> Option<LineTerminator> {
        self.line_term
    }

    fn find_candidate_line(
        &self,
        haystack: &[u8],
    ) -> Result<Option<LineMatchKind>, NoError> {
        if self.every_line_is_candidate {
            assert!(self.line_term.is_some());
            if haystack.is_empty() {
                return Ok(None);
            }
            // Make it interesting and return the last byte in the current
            // line.
            let i = haystack
                .find_byte(self.line_term.unwrap().as_byte())
                .map(|i| i)
                .unwrap_or(haystack.len() - 1);
            Ok(Some(LineMatchKind::Candidate(i)))
        } else {
            Ok(self.shortest_match(haystack)?.map(LineMatchKind::Confirmed))
        }
    }
}

/// An implementation of Sink that prints all available information.
///
/// This is useful for tests because it lets us easily confirm whether data
/// is being passed to Sink correctly.
#[derive(Clone, Debug)]
pub(crate) struct KitchenSink(Vec<u8>);

impl KitchenSink {
    /// Create a new implementation of Sink that includes everything in the
    /// kitchen.
    pub(crate) fn new() -> KitchenSink {
        KitchenSink(vec![])
    }

    /// Return the data written to this sink.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Sink for KitchenSink {
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        assert!(!mat.bytes().is_empty());
        assert!(mat.lines().count() >= 1);

        let mut line_number = mat.line_number();
        let mut byte_offset = mat.absolute_byte_offset();
        for line in mat.lines() {
            if let Some(ref mut n) = line_number {
                write!(self.0, "{}:", n)?;
                *n += 1;
            }

            write!(self.0, "{}:", byte_offset)?;
            byte_offset += line.len() as u64;
            self.0.write_all(line)?;
        }
        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, io::Error> {
        assert!(!context.bytes().is_empty());
        assert!(context.lines().count() == 1);

        if let Some(line_number) = context.line_number() {
            write!(self.0, "{}-", line_number)?;
        }
        write!(self.0, "{}-", context.absolute_byte_offset)?;
        self.0.write_all(context.bytes())?;
        Ok(true)
    }

    fn context_break(
        &mut self,
        _searcher: &Searcher,
    ) -> Result<bool, io::Error> {
        self.0.write_all(b"--\n")?;
        Ok(true)
    }

    fn finish(
        &mut self,
        _searcher: &Searcher,
        sink_finish: &SinkFinish,
    ) -> Result<(), io::Error> {
        writeln!(self.0, "")?;
        writeln!(self.0, "byte count:{}", sink_finish.byte_count())?;
        if let Some(offset) = sink_finish.binary_byte_offset() {
            writeln!(self.0, "binary offset:{}", offset)?;
        }
        Ok(())
    }
}

/// A type for expressing tests on a searcher.
///
/// The searcher code has a lot of different code paths, mostly for the
/// purposes of optimizing a bunch of different use cases. The intent of the
/// searcher is to pick the best code path based on the configuration, which
/// means there is no obviously direct way to ask that a specific code path
/// be exercised. Thus, the purpose of this tester is to explicitly check as
/// many code paths that make sense.
///
/// The tester works by assuming you want to test all pertinent code paths.
/// These can be trimmed down as necessary via the various builder methods.
#[derive(Debug)]
pub(crate) struct SearcherTester {
    haystack: String,
    pattern: String,
    filter: Option<::regex::Regex>,
    print_labels: bool,
    expected_no_line_number: Option<String>,
    expected_with_line_number: Option<String>,
    expected_slice_no_line_number: Option<String>,
    expected_slice_with_line_number: Option<String>,
    by_line: bool,
    multi_line: bool,
    invert_match: bool,
    line_number: bool,
    binary: BinaryDetection,
    auto_heap_limit: bool,
    after_context: usize,
    before_context: usize,
    passthru: bool,
}

impl SearcherTester {
    /// Create a new tester for testing searchers.
    pub(crate) fn new(haystack: &str, pattern: &str) -> SearcherTester {
        SearcherTester {
            haystack: haystack.to_string(),
            pattern: pattern.to_string(),
            filter: None,
            print_labels: false,
            expected_no_line_number: None,
            expected_with_line_number: None,
            expected_slice_no_line_number: None,
            expected_slice_with_line_number: None,
            by_line: true,
            multi_line: true,
            invert_match: false,
            line_number: true,
            binary: BinaryDetection::none(),
            auto_heap_limit: true,
            after_context: 0,
            before_context: 0,
            passthru: false,
        }
    }

    /// Execute the test. If the test succeeds, then this returns successfully.
    /// If the test fails, then it panics with an informative message.
    pub(crate) fn test(&self) {
        // Check for configuration errors.
        if self.expected_no_line_number.is_none() {
            panic!("an 'expected' string with NO line numbers must be given");
        }
        if self.line_number && self.expected_with_line_number.is_none() {
            panic!(
                "an 'expected' string with line numbers must be given, \
                    or disable testing with line numbers"
            );
        }

        let configs = self.configs();
        if configs.is_empty() {
            panic!("test configuration resulted in nothing being tested");
        }
        if self.print_labels {
            for config in &configs {
                let labels = vec![
                    format!("reader-{}", config.label),
                    format!("slice-{}", config.label),
                ];
                for label in &labels {
                    if self.include(label) {
                        println!("{}", label);
                    } else {
                        println!("{} (ignored)", label);
                    }
                }
            }
        }
        for config in &configs {
            let label = format!("reader-{}", config.label);
            if self.include(&label) {
                let got = config.search_reader(&self.haystack);
                assert_eq_printed!(config.expected_reader, got, "{}", label);
            }

            let label = format!("slice-{}", config.label);
            if self.include(&label) {
                let got = config.search_slice(&self.haystack);
                assert_eq_printed!(config.expected_slice, got, "{}", label);
            }
        }
    }

    /// Set a regex pattern to filter the tests that are run.
    ///
    /// By default, no filter is present. When a filter is set, only test
    /// configurations with a label matching the given pattern will be run.
    ///
    /// This is often useful when debugging tests, e.g., when you want to do
    /// printf debugging and only want one particular test configuration to
    /// execute.
    #[allow(dead_code)]
    pub(crate) fn filter(&mut self, pattern: &str) -> &mut SearcherTester {
        self.filter = Some(::regex::Regex::new(pattern).unwrap());
        self
    }

    /// When set, the labels for all test configurations are printed before
    /// executing any test.
    ///
    /// Note that in order to see these in tests that aren't failing, you'll
    /// want to use `cargo test -- --nocapture`.
    #[allow(dead_code)]
    pub(crate) fn print_labels(&mut self, yes: bool) -> &mut SearcherTester {
        self.print_labels = yes;
        self
    }

    /// Set the expected search results, without line numbers.
    pub(crate) fn expected_no_line_number(
        &mut self,
        exp: &str,
    ) -> &mut SearcherTester {
        self.expected_no_line_number = Some(exp.to_string());
        self
    }

    /// Set the expected search results, with line numbers.
    pub(crate) fn expected_with_line_number(
        &mut self,
        exp: &str,
    ) -> &mut SearcherTester {
        self.expected_with_line_number = Some(exp.to_string());
        self
    }

    /// Set the expected search results, without line numbers, when performing
    /// a search on a slice. When not present, `expected_no_line_number` is
    /// used instead.
    pub(crate) fn expected_slice_no_line_number(
        &mut self,
        exp: &str,
    ) -> &mut SearcherTester {
        self.expected_slice_no_line_number = Some(exp.to_string());
        self
    }

    /// Set the expected search results, with line numbers, when performing a
    /// search on a slice. When not present, `expected_with_line_number` is
    /// used instead.
    #[allow(dead_code)]
    pub(crate) fn expected_slice_with_line_number(
        &mut self,
        exp: &str,
    ) -> &mut SearcherTester {
        self.expected_slice_with_line_number = Some(exp.to_string());
        self
    }

    /// Whether to test search with line numbers or not.
    ///
    /// This is enabled by default. When enabled, the string that is expected
    /// when line numbers are present must be provided. Otherwise, the expected
    /// string isn't required.
    pub(crate) fn line_number(&mut self, yes: bool) -> &mut SearcherTester {
        self.line_number = yes;
        self
    }

    /// Whether to test search using the line-by-line searcher or not.
    ///
    /// By default, this is enabled.
    pub(crate) fn by_line(&mut self, yes: bool) -> &mut SearcherTester {
        self.by_line = yes;
        self
    }

    /// Whether to test search using the multi line searcher or not.
    ///
    /// By default, this is enabled.
    #[allow(dead_code)]
    pub(crate) fn multi_line(&mut self, yes: bool) -> &mut SearcherTester {
        self.multi_line = yes;
        self
    }

    /// Whether to perform an inverted search or not.
    ///
    /// By default, this is disabled.
    pub(crate) fn invert_match(&mut self, yes: bool) -> &mut SearcherTester {
        self.invert_match = yes;
        self
    }

    /// Whether to enable binary detection on all searches.
    ///
    /// By default, this is disabled.
    pub(crate) fn binary_detection(
        &mut self,
        detection: BinaryDetection,
    ) -> &mut SearcherTester {
        self.binary = detection;
        self
    }

    /// Whether to automatically attempt to test the heap limit setting or not.
    ///
    /// By default, one of the test configurations includes setting the heap
    /// limit to its minimal value for normal operation, which checks that
    /// everything works even at the extremes. However, in some cases, the heap
    /// limit can (expectedly) alter the output slightly. For example, it can
    /// impact the number of bytes searched when performing binary detection.
    /// For convenience, it can be useful to disable the automatic heap limit
    /// test.
    pub(crate) fn auto_heap_limit(
        &mut self,
        yes: bool,
    ) -> &mut SearcherTester {
        self.auto_heap_limit = yes;
        self
    }

    /// Set the number of lines to include in the "after" context.
    ///
    /// The default is `0`, which is equivalent to not printing any context.
    pub(crate) fn after_context(
        &mut self,
        lines: usize,
    ) -> &mut SearcherTester {
        self.after_context = lines;
        self
    }

    /// Set the number of lines to include in the "before" context.
    ///
    /// The default is `0`, which is equivalent to not printing any context.
    pub(crate) fn before_context(
        &mut self,
        lines: usize,
    ) -> &mut SearcherTester {
        self.before_context = lines;
        self
    }

    /// Whether to enable the "passthru" feature or not.
    ///
    /// When passthru is enabled, it effectively treats all non-matching lines
    /// as contextual lines. In other words, enabling this is akin to
    /// requesting an unbounded number of before and after contextual lines.
    ///
    /// This is disabled by default.
    pub(crate) fn passthru(&mut self, yes: bool) -> &mut SearcherTester {
        self.passthru = yes;
        self
    }

    /// Return the minimum size of a buffer required for a successful search.
    ///
    /// Generally, this corresponds to the maximum length of a line (including
    /// its terminator), but if context settings are enabled, then this must
    /// include the sum of the longest N lines.
    ///
    /// Note that this must account for whether the test is using multi line
    /// search or not, since multi line search requires being able to fit the
    /// entire haystack into memory.
    fn minimal_heap_limit(&self, multi_line: bool) -> usize {
        if multi_line {
            1 + self.haystack.len()
        } else if self.before_context == 0 && self.after_context == 0 {
            1 + self.haystack.lines().map(|s| s.len()).max().unwrap_or(0)
        } else {
            let mut lens: Vec<usize> =
                self.haystack.lines().map(|s| s.len()).collect();
            lens.sort();
            lens.reverse();

            let context_count = if self.passthru {
                self.haystack.lines().count()
            } else {
                // Why do we add 2 here? Well, we need to add 1 in order to
                // have room to search at least one line. We add another
                // because the implementation will occasionally include
                // an additional line when handling the context. There's
                // no particularly good reason, other than keeping the
                // implementation simple.
                2 + self.before_context + self.after_context
            };

            // We add 1 to each line since `str::lines` doesn't include the
            // line terminator.
            lens.into_iter()
                .take(context_count)
                .map(|len| len + 1)
                .sum::<usize>()
        }
    }

    /// Returns true if and only if the given label should be included as part
    /// of executing `test`.
    ///
    /// Inclusion is determined by the filter specified. If no filter has been
    /// given, then this always returns `true`.
    fn include(&self, label: &str) -> bool {
        let re = match self.filter {
            None => return true,
            Some(ref re) => re,
        };
        re.is_match(label)
    }

    /// Configs generates a set of all search configurations that should be
    /// tested. The configs generated are based on the configuration in this
    /// builder.
    fn configs(&self) -> Vec<TesterConfig> {
        let mut configs = vec![];

        let matcher = RegexMatcher::new(&self.pattern);
        let mut builder = SearcherBuilder::new();
        builder
            .line_number(false)
            .invert_match(self.invert_match)
            .binary_detection(self.binary.clone())
            .after_context(self.after_context)
            .before_context(self.before_context)
            .passthru(self.passthru);

        if self.by_line {
            let mut matcher = matcher.clone();
            let mut builder = builder.clone();

            let expected_reader =
                self.expected_no_line_number.as_ref().unwrap().to_string();
            let expected_slice = match self.expected_slice_no_line_number {
                None => expected_reader.clone(),
                Some(ref e) => e.to_string(),
            };
            configs.push(TesterConfig {
                label: "byline-noterm-nonumber".to_string(),
                expected_reader: expected_reader.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });

            if self.auto_heap_limit {
                builder.heap_limit(Some(self.minimal_heap_limit(false)));
                configs.push(TesterConfig {
                    label: "byline-noterm-nonumber-heaplimit".to_string(),
                    expected_reader: expected_reader.clone(),
                    expected_slice: expected_slice.clone(),
                    builder: builder.clone(),
                    matcher: matcher.clone(),
                });
                builder.heap_limit(None);
            }

            matcher.set_line_term(Some(LineTerminator::byte(b'\n')));
            configs.push(TesterConfig {
                label: "byline-term-nonumber".to_string(),
                expected_reader: expected_reader.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });

            matcher.every_line_is_candidate(true);
            configs.push(TesterConfig {
                label: "byline-term-nonumber-candidates".to_string(),
                expected_reader: expected_reader.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });
        }
        if self.by_line && self.line_number {
            let mut matcher = matcher.clone();
            let mut builder = builder.clone();

            let expected_reader =
                self.expected_with_line_number.as_ref().unwrap().to_string();
            let expected_slice = match self.expected_slice_with_line_number {
                None => expected_reader.clone(),
                Some(ref e) => e.to_string(),
            };

            builder.line_number(true);
            configs.push(TesterConfig {
                label: "byline-noterm-number".to_string(),
                expected_reader: expected_reader.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });

            matcher.set_line_term(Some(LineTerminator::byte(b'\n')));
            configs.push(TesterConfig {
                label: "byline-term-number".to_string(),
                expected_reader: expected_reader.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });

            matcher.every_line_is_candidate(true);
            configs.push(TesterConfig {
                label: "byline-term-number-candidates".to_string(),
                expected_reader: expected_reader.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });
        }
        if self.multi_line {
            let mut builder = builder.clone();
            let expected_slice = match self.expected_slice_no_line_number {
                None => {
                    self.expected_no_line_number.as_ref().unwrap().to_string()
                }
                Some(ref e) => e.to_string(),
            };

            builder.multi_line(true);
            configs.push(TesterConfig {
                label: "multiline-nonumber".to_string(),
                expected_reader: expected_slice.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });

            if self.auto_heap_limit {
                builder.heap_limit(Some(self.minimal_heap_limit(true)));
                configs.push(TesterConfig {
                    label: "multiline-nonumber-heaplimit".to_string(),
                    expected_reader: expected_slice.clone(),
                    expected_slice: expected_slice.clone(),
                    builder: builder.clone(),
                    matcher: matcher.clone(),
                });
                builder.heap_limit(None);
            }
        }
        if self.multi_line && self.line_number {
            let mut builder = builder.clone();
            let expected_slice = match self.expected_slice_with_line_number {
                None => self
                    .expected_with_line_number
                    .as_ref()
                    .unwrap()
                    .to_string(),
                Some(ref e) => e.to_string(),
            };

            builder.multi_line(true);
            builder.line_number(true);
            configs.push(TesterConfig {
                label: "multiline-number".to_string(),
                expected_reader: expected_slice.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });

            builder.heap_limit(Some(self.minimal_heap_limit(true)));
            configs.push(TesterConfig {
                label: "multiline-number-heaplimit".to_string(),
                expected_reader: expected_slice.clone(),
                expected_slice: expected_slice.clone(),
                builder: builder.clone(),
                matcher: matcher.clone(),
            });
            builder.heap_limit(None);
        }
        configs
    }
}

#[derive(Debug)]
struct TesterConfig {
    label: String,
    expected_reader: String,
    expected_slice: String,
    builder: SearcherBuilder,
    matcher: RegexMatcher,
}

impl TesterConfig {
    /// Execute a search using a reader. This exercises the incremental search
    /// strategy, where the entire contents of the corpus aren't necessarily
    /// in memory at once.
    fn search_reader(&self, haystack: &str) -> String {
        let mut sink = KitchenSink::new();
        let mut searcher = self.builder.build();
        let result = searcher.search_reader(
            &self.matcher,
            haystack.as_bytes(),
            &mut sink,
        );
        if let Err(err) = result {
            let label = format!("reader-{}", self.label);
            panic!("error running '{}': {}", label, err);
        }
        String::from_utf8(sink.as_bytes().to_vec()).unwrap()
    }

    /// Execute a search using a slice. This exercises the search routines that
    /// have the entire contents of the corpus in memory at one time.
    fn search_slice(&self, haystack: &str) -> String {
        let mut sink = KitchenSink::new();
        let mut searcher = self.builder.build();
        let result = searcher.search_slice(
            &self.matcher,
            haystack.as_bytes(),
            &mut sink,
        );
        if let Err(err) = result {
            let label = format!("slice-{}", self.label);
            panic!("error running '{}': {}", label, err);
        }
        String::from_utf8(sink.as_bytes().to_vec()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use grep_matcher::{Match, Matcher};

    use super::*;

    fn m(start: usize, end: usize) -> Match {
        Match::new(start, end)
    }

    #[test]
    fn empty_line1() {
        let haystack = b"";
        let matcher = RegexMatcher::new(r"^$");

        assert_eq!(matcher.find_at(haystack, 0), Ok(Some(m(0, 0))));
    }

    #[test]
    fn empty_line2() {
        let haystack = b"\n";
        let matcher = RegexMatcher::new(r"^$");

        assert_eq!(matcher.find_at(haystack, 0), Ok(Some(m(0, 0))));
        assert_eq!(matcher.find_at(haystack, 1), Ok(Some(m(1, 1))));
    }

    #[test]
    fn empty_line3() {
        let haystack = b"\n\n";
        let matcher = RegexMatcher::new(r"^$");

        assert_eq!(matcher.find_at(haystack, 0), Ok(Some(m(0, 0))));
        assert_eq!(matcher.find_at(haystack, 1), Ok(Some(m(1, 1))));
        assert_eq!(matcher.find_at(haystack, 2), Ok(Some(m(2, 2))));
    }

    #[test]
    fn empty_line4() {
        let haystack = b"a\n\nb\n";
        let matcher = RegexMatcher::new(r"^$");

        assert_eq!(matcher.find_at(haystack, 0), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 1), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 2), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 3), Ok(Some(m(5, 5))));
        assert_eq!(matcher.find_at(haystack, 4), Ok(Some(m(5, 5))));
        assert_eq!(matcher.find_at(haystack, 5), Ok(Some(m(5, 5))));
    }

    #[test]
    fn empty_line5() {
        let haystack = b"a\n\nb\nc";
        let matcher = RegexMatcher::new(r"^$");

        assert_eq!(matcher.find_at(haystack, 0), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 1), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 2), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 3), Ok(None));
        assert_eq!(matcher.find_at(haystack, 4), Ok(None));
        assert_eq!(matcher.find_at(haystack, 5), Ok(None));
        assert_eq!(matcher.find_at(haystack, 6), Ok(None));
    }

    #[test]
    fn empty_line6() {
        let haystack = b"a\n";
        let matcher = RegexMatcher::new(r"^$");

        assert_eq!(matcher.find_at(haystack, 0), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 1), Ok(Some(m(2, 2))));
        assert_eq!(matcher.find_at(haystack, 2), Ok(Some(m(2, 2))));
    }
}
