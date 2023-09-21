use std::{
    io::{self, Write},
    path::Path,
    time::Instant,
};

use {
    grep_matcher::{Match, Matcher},
    grep_searcher::{
        Searcher, Sink, SinkContext, SinkContextKind, SinkFinish, SinkMatch,
    },
    serde_json as json,
};

use crate::{
    counter::CounterWriter, jsont, stats::Stats, util::find_iter_at_in_context,
};

/// The configuration for the JSON printer.
///
/// This is manipulated by the JSONBuilder and then referenced by the actual
/// implementation. Once a printer is build, the configuration is frozen and
/// cannot changed.
#[derive(Debug, Clone)]
struct Config {
    pretty: bool,
    max_matches: Option<u64>,
    always_begin_end: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { pretty: false, max_matches: None, always_begin_end: false }
    }
}

/// A builder for a JSON lines printer.
///
/// The builder permits configuring how the printer behaves. The JSON printer
/// has fewer configuration options than the standard printer because it is
/// a structured format, and the printer always attempts to find the most
/// information possible.
///
/// Some configuration options, such as whether line numbers are included or
/// whether contextual lines are shown, are drawn directly from the
/// `grep_searcher::Searcher`'s configuration.
///
/// Once a `JSON` printer is built, its configuration cannot be changed.
#[derive(Clone, Debug)]
pub struct JSONBuilder {
    config: Config,
}

impl JSONBuilder {
    /// Return a new builder for configuring the JSON printer.
    pub fn new() -> JSONBuilder {
        JSONBuilder { config: Config::default() }
    }

    /// Create a JSON printer that writes results to the given writer.
    pub fn build<W: io::Write>(&self, wtr: W) -> JSON<W> {
        JSON {
            config: self.config.clone(),
            wtr: CounterWriter::new(wtr),
            matches: vec![],
        }
    }

    /// Print JSON in a pretty printed format.
    ///
    /// Enabling this will no longer produce a "JSON lines" format, in that
    /// each JSON object printed may span multiple lines.
    ///
    /// This is disabled by default.
    pub fn pretty(&mut self, yes: bool) -> &mut JSONBuilder {
        self.config.pretty = yes;
        self
    }

    /// Set the maximum amount of matches that are printed.
    ///
    /// If multi line search is enabled and a match spans multiple lines, then
    /// that match is counted exactly once for the purposes of enforcing this
    /// limit, regardless of how many lines it spans.
    pub fn max_matches(&mut self, limit: Option<u64>) -> &mut JSONBuilder {
        self.config.max_matches = limit;
        self
    }

    /// When enabled, the `begin` and `end` messages are always emitted, even
    /// when no match is found.
    ///
    /// When disabled, the `begin` and `end` messages are only shown if there
    /// is at least one `match` or `context` message.
    ///
    /// This is disabled by default.
    pub fn always_begin_end(&mut self, yes: bool) -> &mut JSONBuilder {
        self.config.always_begin_end = yes;
        self
    }
}

/// The JSON printer, which emits results in a JSON lines format.
///
/// This type is generic over `W`, which represents any implementation of
/// the standard library `io::Write` trait.
///
/// # Format
///
/// This section describes the JSON format used by this printer.
///
/// To skip the rigamarole, take a look at the
/// [example](#example)
/// at the end.
///
/// ## Overview
///
/// The format of this printer is the [JSON Lines](https://jsonlines.org/)
/// format. Specifically, this printer emits a sequence of messages, where
/// each message is encoded as a single JSON value on a single line. There are
/// four different types of messages (and this number may expand over time):
///
/// * **begin** - A message that indicates a file is being searched.
/// * **end** - A message the indicates a file is done being searched. This
///   message also include summary statistics about the search.
/// * **match** - A message that indicates a match was found. This includes
///   the text and offsets of the match.
/// * **context** - A message that indicates a contextual line was found.
///   This includes the text of the line, along with any match information if
///   the search was inverted.
///
/// Every message is encoded in the same envelope format, which includes a tag
/// indicating the message type along with an object for the payload:
///
/// ```json
/// {
///     "type": "{begin|end|match|context}",
///     "data": { ... }
/// }
/// ```
///
/// The message itself is encoded in the envelope's `data` key.
///
/// ## Text encoding
///
/// Before describing each message format, we first must briefly discuss text
/// encoding, since it factors into every type of message. In particular, JSON
/// may only be encoded in UTF-8, UTF-16 or UTF-32. For the purposes of this
/// printer, we need only worry about UTF-8. The problem here is that searching
/// is not limited to UTF-8 exclusively, which in turn implies that matches
/// may be reported that contain invalid UTF-8. Moreover, this printer may
/// also print file paths, and the encoding of file paths is itself not
/// guaranteed to be valid UTF-8. Therefore, this printer must deal with the
/// presence of invalid UTF-8 somehow. The printer could silently ignore such
/// things completely, or even lossily transcode invalid UTF-8 to valid UTF-8
/// by replacing all invalid sequences with the Unicode replacement character.
/// However, this would prevent consumers of this format from accessing the
/// original data in a non-lossy way.
///
/// Therefore, this printer will emit valid UTF-8 encoded bytes as normal
/// JSON strings and otherwise base64 encode data that isn't valid UTF-8. To
/// communicate whether this process occurs or not, strings are keyed by the
/// name `text` where as arbitrary bytes are keyed by `bytes`.
///
/// For example, when a path is included in a message, it is formatted like so,
/// if and only if the path is valid UTF-8:
///
/// ```json
/// {
///     "path": {
///         "text": "/home/ubuntu/lib.rs"
///     }
/// }
/// ```
///
/// If instead our path was `/home/ubuntu/lib\xFF.rs`, where the `\xFF` byte
/// makes it invalid UTF-8, the path would instead be encoded like so:
///
/// ```json
/// {
///     "path": {
///         "bytes": "L2hvbWUvdWJ1bnR1L2xpYv8ucnM="
///     }
/// }
/// ```
///
/// This same representation is used for reporting matches as well.
///
/// The printer guarantees that the `text` field is used whenever the
/// underlying bytes are valid UTF-8.
///
/// ## Wire format
///
/// This section documents the wire format emitted by this printer, starting
/// with the four types of messages.
///
/// Each message has its own format, and is contained inside an envelope that
/// indicates the type of message. The envelope has these fields:
///
/// * **type** - A string indicating the type of this message. It may be one
///   of four possible strings: `begin`, `end`, `match` or `context`. This
///   list may expand over time.
/// * **data** - The actual message data. The format of this field depends on
///   the value of `type`. The possible message formats are
///   [`begin`](#message-begin),
///   [`end`](#message-end),
///   [`match`](#message-match),
///   [`context`](#message-context).
///
/// #### Message: **begin**
///
/// This message indicates that a search has begun. It has these fields:
///
/// * **path** - An
///   [arbitrary data object](#object-arbitrary-data)
///   representing the file path corresponding to the search, if one is
///   present. If no file path is available, then this field is `null`.
///
/// #### Message: **end**
///
/// This message indicates that a search has finished. It has these fields:
///
/// * **path** - An
///   [arbitrary data object](#object-arbitrary-data)
///   representing the file path corresponding to the search, if one is
///   present. If no file path is available, then this field is `null`.
/// * **binary_offset** - The absolute offset in the data searched
///   corresponding to the place at which binary data was detected. If no
///   binary data was detected (or if binary detection was disabled), then this
///   field is `null`.
/// * **stats** - A [`stats` object](#object-stats) that contains summary
///   statistics for the previous search.
///
/// #### Message: **match**
///
/// This message indicates that a match has been found. A match generally
/// corresponds to a single line of text, although it may correspond to
/// multiple lines if the search can emit matches over multiple lines. It
/// has these fields:
///
/// * **path** - An
///   [arbitrary data object](#object-arbitrary-data)
///   representing the file path corresponding to the search, if one is
///   present. If no file path is available, then this field is `null`.
/// * **lines** - An
///   [arbitrary data object](#object-arbitrary-data)
///   representing one or more lines contained in this match.
/// * **line_number** - If the searcher has been configured to report line
///   numbers, then this corresponds to the line number of the first line
///   in `lines`. If no line numbers are available, then this is `null`.
/// * **absolute_offset** - The absolute byte offset corresponding to the start
///   of `lines` in the data being searched.
/// * **submatches** - An array of [`submatch` objects](#object-submatch)
///   corresponding to matches in `lines`. The offsets included in each
///   `submatch` correspond to byte offsets into `lines`. (If `lines` is base64
///   encoded, then the byte offsets correspond to the data after base64
///   decoding.) The `submatch` objects are guaranteed to be sorted by their
///   starting offsets. Note that it is possible for this array to be empty,
///   for example, when searching reports inverted matches.
///
/// #### Message: **context**
///
/// This message indicates that a contextual line has been found. A contextual
/// line is a line that doesn't contain a match, but is generally adjacent to
/// a line that does contain a match. The precise way in which contextual lines
/// are reported is determined by the searcher. It has these fields, which are
/// exactly the same fields found in a [`match`](#message-match):
///
/// * **path** - An
///   [arbitrary data object](#object-arbitrary-data)
///   representing the file path corresponding to the search, if one is
///   present. If no file path is available, then this field is `null`.
/// * **lines** - An
///   [arbitrary data object](#object-arbitrary-data)
///   representing one or more lines contained in this context. This includes
///   line terminators, if they're present.
/// * **line_number** - If the searcher has been configured to report line
///   numbers, then this corresponds to the line number of the first line
///   in `lines`. If no line numbers are available, then this is `null`.
/// * **absolute_offset** - The absolute byte offset corresponding to the start
///   of `lines` in the data being searched.
/// * **submatches** - An array of [`submatch` objects](#object-submatch)
///   corresponding to matches in `lines`. The offsets included in each
///   `submatch` correspond to byte offsets into `lines`. (If `lines` is base64
///   encoded, then the byte offsets correspond to the data after base64
///   decoding.) The `submatch` objects are guaranteed to be sorted by
///   their starting offsets. Note that it is possible for this array to be
///   non-empty, for example, when searching reports inverted matches such that
///   the original matcher could match things in the contextual lines.
///
/// #### Object: **submatch**
///
/// This object describes submatches found within `match` or `context`
/// messages. The `start` and `end` fields indicate the half-open interval on
/// which the match occurs (`start` is included, but `end` is not). It is
/// guaranteed that `start <= end`. It has these fields:
///
/// * **match** - An
///   [arbitrary data object](#object-arbitrary-data)
///   corresponding to the text in this submatch.
/// * **start** - A byte offset indicating the start of this match. This offset
///   is generally reported in terms of the parent object's data. For example,
///   the `lines` field in the
///   [`match`](#message-match) or [`context`](#message-context)
///   messages.
/// * **end** - A byte offset indicating the end of this match. This offset
///   is generally reported in terms of the parent object's data. For example,
///   the `lines` field in the
///   [`match`](#message-match) or [`context`](#message-context)
///   messages.
///
/// #### Object: **stats**
///
/// This object is included in messages and contains summary statistics about
/// a search. It has these fields:
///
/// * **elapsed** - A [`duration` object](#object-duration) describing the
///   length of time that elapsed while performing the search.
/// * **searches** - The number of searches that have run. For this printer,
///   this value is always `1`. (Implementations may emit additional message
///   types that use this same `stats` object that represents summary
///   statistics over multiple searches.)
/// * **searches_with_match** - The number of searches that have run that have
///   found at least one match. This is never more than `searches`.
/// * **bytes_searched** - The total number of bytes that have been searched.
/// * **bytes_printed** - The total number of bytes that have been printed.
///   This includes everything emitted by this printer.
/// * **matched_lines** - The total number of lines that participated in a
///   match. When matches may contain multiple lines, then this includes every
///   line that is part of every match.
/// * **matches** - The total number of matches. There may be multiple matches
///   per line. When matches may contain multiple lines, each match is counted
///   only once, regardless of how many lines it spans.
///
/// #### Object: **duration**
///
/// This object includes a few fields for describing a duration. Two of its
/// fields, `secs` and `nanos`, can be combined to give nanosecond precision
/// on systems that support it. It has these fields:
///
/// * **secs** - A whole number of seconds indicating the length of this
///   duration.
/// * **nanos** - A fractional part of this duration represent by nanoseconds.
///   If nanosecond precision isn't supported, then this is typically rounded
///   up to the nearest number of nanoseconds.
/// * **human** - A human readable string describing the length of the
///   duration. The format of the string is itself unspecified.
///
/// #### Object: **arbitrary data**
///
/// This object is used whenever arbitrary data needs to be represented as a
/// JSON value. This object contains two fields, where generally only one of
/// the fields is present:
///
/// * **text** - A normal JSON string that is UTF-8 encoded. This field is
///   populated if and only if the underlying data is valid UTF-8.
/// * **bytes** - A normal JSON string that is a base64 encoding of the
///   underlying bytes.
///
/// More information on the motivation for this representation can be seen in
/// the section [text encoding](#text-encoding) above.
///
/// ## Example
///
/// This section shows a small example that includes all message types.
///
/// Here's the file we want to search, located at `/home/andrew/sherlock`:
///
/// ```text
/// For the Doctor Watsons of this world, as opposed to the Sherlock
/// Holmeses, success in the province of detective work must always
/// be, to a very large extent, the result of luck. Sherlock Holmes
/// can extract a clew from a wisp of straw or a flake of cigar ash;
/// but Doctor Watson has to have it taken out for him and dusted,
/// and exhibited clearly, with a label attached.
/// ```
///
/// Searching for `Watson` with a `before_context` of `1` with line numbers
/// enabled shows something like this using the standard printer:
///
/// ```text
/// sherlock:1:For the Doctor Watsons of this world, as opposed to the Sherlock
/// --
/// sherlock-4-can extract a clew from a wisp of straw or a flake of cigar ash;
/// sherlock:5:but Doctor Watson has to have it taken out for him and dusted,
/// ```
///
/// Here's what the same search looks like using the JSON wire format described
/// above, where in we show semi-prettified JSON (instead of a strict JSON
/// Lines format), for illustrative purposes:
///
/// ```json
/// {
///   "type": "begin",
///   "data": {
///     "path": {"text": "/home/andrew/sherlock"}}
///   }
/// }
/// {
///   "type": "match",
///   "data": {
///     "path": {"text": "/home/andrew/sherlock"},
///     "lines": {"text": "For the Doctor Watsons of this world, as opposed to the Sherlock\n"},
///     "line_number": 1,
///     "absolute_offset": 0,
///     "submatches": [
///       {"match": {"text": "Watson"}, "start": 15, "end": 21}
///     ]
///   }
/// }
/// {
///   "type": "context",
///   "data": {
///     "path": {"text": "/home/andrew/sherlock"},
///     "lines": {"text": "can extract a clew from a wisp of straw or a flake of cigar ash;\n"},
///     "line_number": 4,
///     "absolute_offset": 193,
///     "submatches": []
///   }
/// }
/// {
///   "type": "match",
///   "data": {
///     "path": {"text": "/home/andrew/sherlock"},
///     "lines": {"text": "but Doctor Watson has to have it taken out for him and dusted,\n"},
///     "line_number": 5,
///     "absolute_offset": 258,
///     "submatches": [
///       {"match": {"text": "Watson"}, "start": 11, "end": 17}
///     ]
///   }
/// }
/// {
///   "type": "end",
///   "data": {
///     "path": {"text": "/home/andrew/sherlock"},
///     "binary_offset": null,
///     "stats": {
///       "elapsed": {"secs": 0, "nanos": 36296, "human": "0.0000s"},
///       "searches": 1,
///       "searches_with_match": 1,
///       "bytes_searched": 367,
///       "bytes_printed": 1151,
///       "matched_lines": 2,
///       "matches": 2
///     }
///   }
/// }
/// ```
#[derive(Debug)]
pub struct JSON<W> {
    config: Config,
    wtr: CounterWriter<W>,
    matches: Vec<Match>,
}

impl<W: io::Write> JSON<W> {
    /// Return a JSON lines printer with a default configuration that writes
    /// matches to the given writer.
    pub fn new(wtr: W) -> JSON<W> {
        JSONBuilder::new().build(wtr)
    }

    /// Return an implementation of `Sink` for the JSON printer.
    ///
    /// This does not associate the printer with a file path, which means this
    /// implementation will never print a file path along with the matches.
    pub fn sink<'s, M: Matcher>(
        &'s mut self,
        matcher: M,
    ) -> JSONSink<'static, 's, M, W> {
        JSONSink {
            matcher,
            json: self,
            path: None,
            start_time: Instant::now(),
            match_count: 0,
            after_context_remaining: 0,
            binary_byte_offset: None,
            begin_printed: false,
            stats: Stats::new(),
        }
    }

    /// Return an implementation of `Sink` associated with a file path.
    ///
    /// When the printer is associated with a path, then it may, depending on
    /// its configuration, print the path along with the matches found.
    pub fn sink_with_path<'p, 's, M, P>(
        &'s mut self,
        matcher: M,
        path: &'p P,
    ) -> JSONSink<'p, 's, M, W>
    where
        M: Matcher,
        P: ?Sized + AsRef<Path>,
    {
        JSONSink {
            matcher,
            json: self,
            path: Some(path.as_ref()),
            start_time: Instant::now(),
            match_count: 0,
            after_context_remaining: 0,
            binary_byte_offset: None,
            begin_printed: false,
            stats: Stats::new(),
        }
    }

    /// Write the given message followed by a new line. The new line is
    /// determined from the configuration of the given searcher.
    fn write_message(
        &mut self,
        message: &jsont::Message<'_>,
    ) -> io::Result<()> {
        if self.config.pretty {
            json::to_writer_pretty(&mut self.wtr, message)?;
        } else {
            json::to_writer(&mut self.wtr, message)?;
        }
        self.wtr.write(&[b'\n'])?;
        Ok(())
    }
}

impl<W> JSON<W> {
    /// Returns true if and only if this printer has written at least one byte
    /// to the underlying writer during any of the previous searches.
    pub fn has_written(&self) -> bool {
        self.wtr.total_count() > 0
    }

    /// Return a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.wtr.get_mut()
    }

    /// Consume this printer and return back ownership of the underlying
    /// writer.
    pub fn into_inner(self) -> W {
        self.wtr.into_inner()
    }
}

/// An implementation of `Sink` associated with a matcher and an optional file
/// path for the JSON printer.
///
/// This type is generic over a few type parameters:
///
/// * `'p` refers to the lifetime of the file path, if one is provided. When
///   no file path is given, then this is `'static`.
/// * `'s` refers to the lifetime of the
///   [`JSON`](struct.JSON.html)
///   printer that this type borrows.
/// * `M` refers to the type of matcher used by
///   `grep_searcher::Searcher` that is reporting results to this sink.
/// * `W` refers to the underlying writer that this printer is writing its
///   output to.
#[derive(Debug)]
pub struct JSONSink<'p, 's, M: Matcher, W> {
    matcher: M,
    json: &'s mut JSON<W>,
    path: Option<&'p Path>,
    start_time: Instant,
    match_count: u64,
    after_context_remaining: u64,
    binary_byte_offset: Option<u64>,
    begin_printed: bool,
    stats: Stats,
}

impl<'p, 's, M: Matcher, W: io::Write> JSONSink<'p, 's, M, W> {
    /// Returns true if and only if this printer received a match in the
    /// previous search.
    ///
    /// This is unaffected by the result of searches before the previous
    /// search.
    pub fn has_match(&self) -> bool {
        self.match_count > 0
    }

    /// Return the total number of matches reported to this sink.
    ///
    /// This corresponds to the number of times `Sink::matched` is called.
    pub fn match_count(&self) -> u64 {
        self.match_count
    }

    /// If binary data was found in the previous search, this returns the
    /// offset at which the binary data was first detected.
    ///
    /// The offset returned is an absolute offset relative to the entire
    /// set of bytes searched.
    ///
    /// This is unaffected by the result of searches before the previous
    /// search. e.g., If the search prior to the previous search found binary
    /// data but the previous search found no binary data, then this will
    /// return `None`.
    pub fn binary_byte_offset(&self) -> Option<u64> {
        self.binary_byte_offset
    }

    /// Return a reference to the stats produced by the printer for all
    /// searches executed on this sink.
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Execute the matcher over the given bytes and record the match
    /// locations if the current configuration demands match granularity.
    fn record_matches(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.json.matches.clear();
        // If printing requires knowing the location of each individual match,
        // then compute and stored those right now for use later. While this
        // adds an extra copy for storing the matches, we do amortize the
        // allocation for it and this greatly simplifies the printing logic to
        // the extent that it's easy to ensure that we never do more than
        // one search to find the matches.
        let matches = &mut self.json.matches;
        find_iter_at_in_context(
            searcher,
            &self.matcher,
            bytes,
            range.clone(),
            |m| {
                let (s, e) = (m.start() - range.start, m.end() - range.start);
                matches.push(Match::new(s, e));
                true
            },
        )?;
        // Don't report empty matches appearing at the end of the bytes.
        if !matches.is_empty()
            && matches.last().unwrap().is_empty()
            && matches.last().unwrap().start() >= bytes.len()
        {
            matches.pop().unwrap();
        }
        Ok(())
    }

    /// Returns true if this printer should quit.
    ///
    /// This implements the logic for handling quitting after seeing a certain
    /// amount of matches. In most cases, the logic is simple, but we must
    /// permit all "after" contextual lines to print after reaching the limit.
    fn should_quit(&self) -> bool {
        let limit = match self.json.config.max_matches {
            None => return false,
            Some(limit) => limit,
        };
        if self.match_count < limit {
            return false;
        }
        self.after_context_remaining == 0
    }

    /// Returns whether the current match count exceeds the configured limit.
    /// If there is no limit, then this always returns false.
    fn match_more_than_limit(&self) -> bool {
        let limit = match self.json.config.max_matches {
            None => return false,
            Some(limit) => limit,
        };
        self.match_count > limit
    }

    /// Write the "begin" message.
    fn write_begin_message(&mut self) -> io::Result<()> {
        if self.begin_printed {
            return Ok(());
        }
        let msg = jsont::Message::Begin(jsont::Begin { path: self.path });
        self.json.write_message(&msg)?;
        self.begin_printed = true;
        Ok(())
    }
}

impl<'p, 's, M: Matcher, W: io::Write> Sink for JSONSink<'p, 's, M, W> {
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        self.write_begin_message()?;

        self.match_count += 1;
        // When we've exceeded our match count, then the remaining context
        // lines should not be reset, but instead, decremented. This avoids a
        // bug where we display more matches than a configured limit. The main
        // idea here is that 'matched' might be called again while printing
        // an after-context line. In that case, we should treat this as a
        // contextual line rather than a matching line for the purposes of
        // termination.
        if self.match_more_than_limit() {
            self.after_context_remaining =
                self.after_context_remaining.saturating_sub(1);
        } else {
            self.after_context_remaining = searcher.after_context() as u64;
        }

        self.record_matches(
            searcher,
            mat.buffer(),
            mat.bytes_range_in_buffer(),
        )?;
        self.stats.add_matches(self.json.matches.len() as u64);
        self.stats.add_matched_lines(mat.lines().count() as u64);

        let submatches = SubMatches::new(mat.bytes(), &self.json.matches);
        let msg = jsont::Message::Match(jsont::Match {
            path: self.path,
            lines: mat.bytes(),
            line_number: mat.line_number(),
            absolute_offset: mat.absolute_byte_offset(),
            submatches: submatches.as_slice(),
        });
        self.json.write_message(&msg)?;
        Ok(!self.should_quit())
    }

    fn context(
        &mut self,
        searcher: &Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, io::Error> {
        self.write_begin_message()?;
        self.json.matches.clear();

        if ctx.kind() == &SinkContextKind::After {
            self.after_context_remaining =
                self.after_context_remaining.saturating_sub(1);
        }
        let submatches = if searcher.invert_match() {
            self.record_matches(searcher, ctx.bytes(), 0..ctx.bytes().len())?;
            SubMatches::new(ctx.bytes(), &self.json.matches)
        } else {
            SubMatches::empty()
        };
        let msg = jsont::Message::Context(jsont::Context {
            path: self.path,
            lines: ctx.bytes(),
            line_number: ctx.line_number(),
            absolute_offset: ctx.absolute_byte_offset(),
            submatches: submatches.as_slice(),
        });
        self.json.write_message(&msg)?;
        Ok(!self.should_quit())
    }

    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, io::Error> {
        self.json.wtr.reset_count();
        self.start_time = Instant::now();
        self.match_count = 0;
        self.after_context_remaining = 0;
        self.binary_byte_offset = None;
        if self.json.config.max_matches == Some(0) {
            return Ok(false);
        }

        if !self.json.config.always_begin_end {
            return Ok(true);
        }
        self.write_begin_message()?;
        Ok(true)
    }

    fn finish(
        &mut self,
        _searcher: &Searcher,
        finish: &SinkFinish,
    ) -> Result<(), io::Error> {
        if !self.begin_printed {
            return Ok(());
        }

        self.binary_byte_offset = finish.binary_byte_offset();
        self.stats.add_elapsed(self.start_time.elapsed());
        self.stats.add_searches(1);
        if self.match_count > 0 {
            self.stats.add_searches_with_match(1);
        }
        self.stats.add_bytes_searched(finish.byte_count());
        self.stats.add_bytes_printed(self.json.wtr.count());

        let msg = jsont::Message::End(jsont::End {
            path: self.path,
            binary_offset: finish.binary_byte_offset(),
            stats: self.stats.clone(),
        });
        self.json.write_message(&msg)?;
        Ok(())
    }
}

/// SubMatches represents a set of matches in a contiguous range of bytes.
///
/// A simpler representation for this would just simply be `Vec<SubMatch>`,
/// but the common case is exactly one match per range of bytes, which we
/// specialize here using a fixed size array without any allocation.
enum SubMatches<'a> {
    Empty,
    Small([jsont::SubMatch<'a>; 1]),
    Big(Vec<jsont::SubMatch<'a>>),
}

impl<'a> SubMatches<'a> {
    /// Create a new set of match ranges from a set of matches and the
    /// corresponding bytes that those matches apply to.
    fn new(bytes: &'a [u8], matches: &[Match]) -> SubMatches<'a> {
        if matches.len() == 1 {
            let mat = matches[0];
            SubMatches::Small([jsont::SubMatch {
                m: &bytes[mat],
                start: mat.start(),
                end: mat.end(),
            }])
        } else {
            let mut match_ranges = vec![];
            for &mat in matches {
                match_ranges.push(jsont::SubMatch {
                    m: &bytes[mat],
                    start: mat.start(),
                    end: mat.end(),
                });
            }
            SubMatches::Big(match_ranges)
        }
    }

    /// Create an empty set of match ranges.
    fn empty() -> SubMatches<'static> {
        SubMatches::Empty
    }

    /// Return this set of match ranges as a slice.
    fn as_slice(&self) -> &[jsont::SubMatch<'_>] {
        match *self {
            SubMatches::Empty => &[],
            SubMatches::Small(ref x) => x,
            SubMatches::Big(ref x) => x,
        }
    }
}

#[cfg(test)]
mod tests {
    use grep_matcher::LineTerminator;
    use grep_regex::{RegexMatcher, RegexMatcherBuilder};
    use grep_searcher::SearcherBuilder;

    use super::{JSONBuilder, JSON};

    const SHERLOCK: &'static [u8] = b"\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
can extract a clew from a wisp of straw or a flake of cigar ash;
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.
";

    fn printer_contents(printer: &mut JSON<Vec<u8>>) -> String {
        String::from_utf8(printer.get_mut().to_owned()).unwrap()
    }

    #[test]
    fn binary_detection() {
        use grep_searcher::BinaryDetection;

        const BINARY: &'static [u8] = b"\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
can extract a clew \x00 from a wisp of straw or a flake of cigar ash;
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.\
";

        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer = JSONBuilder::new().build(vec![]);
        SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .heap_limit(Some(80))
            .build()
            .search_reader(&matcher, BINARY, printer.sink(&matcher))
            .unwrap();
        let got = printer_contents(&mut printer);

        assert_eq!(got.lines().count(), 3);
        let last = got.lines().last().unwrap();
        assert!(last.contains(r#""binary_offset":212,"#));
    }

    #[test]
    fn max_matches() {
        let matcher = RegexMatcher::new(r"Watson").unwrap();
        let mut printer =
            JSONBuilder::new().max_matches(Some(1)).build(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(&matcher, SHERLOCK, printer.sink(&matcher))
            .unwrap();
        let got = printer_contents(&mut printer);

        assert_eq!(got.lines().count(), 3);
    }

    #[test]
    fn max_matches_after_context() {
        let haystack = "\
a
b
c
d
e
d
e
d
e
d
e
";
        let matcher = RegexMatcher::new(r"d").unwrap();
        let mut printer =
            JSONBuilder::new().max_matches(Some(1)).build(vec![]);
        SearcherBuilder::new()
            .after_context(2)
            .build()
            .search_reader(
                &matcher,
                haystack.as_bytes(),
                printer.sink(&matcher),
            )
            .unwrap();
        let got = printer_contents(&mut printer);

        assert_eq!(got.lines().count(), 5);
    }

    #[test]
    fn no_match() {
        let matcher = RegexMatcher::new(r"DOES NOT MATCH").unwrap();
        let mut printer = JSONBuilder::new().build(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(&matcher, SHERLOCK, printer.sink(&matcher))
            .unwrap();
        let got = printer_contents(&mut printer);

        assert!(got.is_empty());
    }

    #[test]
    fn always_begin_end_no_match() {
        let matcher = RegexMatcher::new(r"DOES NOT MATCH").unwrap();
        let mut printer =
            JSONBuilder::new().always_begin_end(true).build(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(&matcher, SHERLOCK, printer.sink(&matcher))
            .unwrap();
        let got = printer_contents(&mut printer);

        assert_eq!(got.lines().count(), 2);
        assert!(got.contains("begin") && got.contains("end"));
    }

    #[test]
    fn missing_crlf() {
        let haystack = "test\r\n".as_bytes();

        let matcher = RegexMatcherBuilder::new().build("test").unwrap();
        let mut printer = JSONBuilder::new().build(vec![]);
        SearcherBuilder::new()
            .build()
            .search_reader(&matcher, haystack, printer.sink(&matcher))
            .unwrap();
        let got = printer_contents(&mut printer);
        assert_eq!(got.lines().count(), 3);
        assert!(
            got.lines().nth(1).unwrap().contains(r"test\r\n"),
            r"missing 'test\r\n' in '{}'",
            got.lines().nth(1).unwrap(),
        );

        let matcher =
            RegexMatcherBuilder::new().crlf(true).build("test").unwrap();
        let mut printer = JSONBuilder::new().build(vec![]);
        SearcherBuilder::new()
            .line_terminator(LineTerminator::crlf())
            .build()
            .search_reader(&matcher, haystack, printer.sink(&matcher))
            .unwrap();
        let got = printer_contents(&mut printer);
        assert_eq!(got.lines().count(), 3);
        assert!(
            got.lines().nth(1).unwrap().contains(r"test\r\n"),
            r"missing 'test\r\n' in '{}'",
            got.lines().nth(1).unwrap(),
        );
    }
}
