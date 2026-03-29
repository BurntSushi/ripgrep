use std::{io, io::Write, path::Path, time::Instant};

use grep_matcher::Matcher;
use grep_searcher::{Searcher, Sink, SinkContext, SinkFinish, SinkMatch};

use crate::{
    counter::CounterWriter,
    jsont,
    stats::Stats,
    util::{
        CaptureMatch as RecordedCaptureMatch, Replacer,
        capture_matches_in_context,
    },
};

#[derive(Clone, Debug, Default)]
struct Config {
    pretty: bool,
    always_begin_end: bool,
    lines: bool,
}

/// A builder for configuring JSON capture output.
#[derive(Clone, Debug)]
pub struct JSONCapturesBuilder {
    config: Config,
}

impl JSONCapturesBuilder {
    /// Return a new builder with the default JSON capture configuration.
    pub fn new() -> JSONCapturesBuilder {
        JSONCapturesBuilder { config: Config::default() }
    }

    /// Pretty-print JSON output.
    pub fn pretty(&mut self, yes: bool) -> &mut JSONCapturesBuilder {
        self.config.pretty = yes;
        self
    }

    /// Emit `begin` and `end` messages even when a search has no matches.
    pub fn always_begin_end(&mut self, yes: bool) -> &mut JSONCapturesBuilder {
        self.config.always_begin_end = yes;
        self
    }

    /// Include the parent `lines` payload in each match or context message.
    pub fn lines(&mut self, yes: bool) -> &mut JSONCapturesBuilder {
        self.config.lines = yes;
        self
    }

    /// Build a JSON capture printer from the current configuration.
    pub fn build<W: io::Write>(&self, wtr: W) -> JSONCaptures<W> {
        JSONCaptures {
            config: self.config.clone(),
            wtr: CounterWriter::new(wtr),
            occurrences: vec![],
        }
    }
}

/// A JSON Lines printer that emits full regex occurrences and capture groups.
#[derive(Clone, Debug)]
pub struct JSONCaptures<W> {
    config: Config,
    wtr: CounterWriter<W>,
    occurrences: Vec<RecordedCaptureMatch>,
}

impl<W: io::Write> JSONCaptures<W> {
    /// Return a JSON capture printer with the default configuration.
    pub fn new(wtr: W) -> JSONCaptures<W> {
        JSONCapturesBuilder::new().build(wtr)
    }

    /// Return an implementation of `Sink` for this printer.
    pub fn sink<'s, M: Matcher>(
        &'s mut self,
        matcher: M,
    ) -> JSONCapturesSink<'static, 's, M, W> {
        JSONCapturesSink {
            matcher,
            scratch: Replacer::new(),
            json: self,
            path: None,
            start_time: Instant::now(),
            match_count: 0,
            binary_byte_offset: None,
            begin_printed: false,
            stats: Stats::new(),
        }
    }

    /// Return an implementation of `Sink` associated with a file path.
    pub fn sink_with_path<'p, 's, M, P>(
        &'s mut self,
        matcher: M,
        path: &'p P,
    ) -> JSONCapturesSink<'p, 's, M, W>
    where
        M: Matcher,
        P: ?Sized + AsRef<Path>,
    {
        JSONCapturesSink {
            matcher,
            scratch: Replacer::new(),
            json: self,
            path: Some(path.as_ref()),
            start_time: Instant::now(),
            match_count: 0,
            binary_byte_offset: None,
            begin_printed: false,
            stats: Stats::new(),
        }
    }

    /// Return whether this printer requires explicit capture groups.
    /// Returns true if and only if this printer requires explicit capture
    /// groups.
    pub fn requires_explicit_captures(&self) -> bool {
        true
    }

    /// Return a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.wtr.get_mut()
    }

    fn write_message(
        &mut self,
        message: &jsont::CaptureMessage<'_>,
    ) -> io::Result<()> {
        if self.config.pretty {
            serde_json::to_writer_pretty(&mut self.wtr, message)?;
        } else {
            serde_json::to_writer(&mut self.wtr, message)?;
        }
        self.wtr.write_all(b"\n")?;
        Ok(())
    }
}

/// A sink for the JSON capture printer.
#[derive(Debug)]
pub struct JSONCapturesSink<'p, 's, M: Matcher, W> {
    matcher: M,
    scratch: Replacer<M>,
    json: &'s mut JSONCaptures<W>,
    path: Option<&'p Path>,
    start_time: Instant,
    match_count: u64,
    binary_byte_offset: Option<u64>,
    begin_printed: bool,
    stats: Stats,
}

impl<'p, 's, M: Matcher, W: io::Write> JSONCapturesSink<'p, 's, M, W> {
    /// Returns true if and only if this sink saw a match in the previous
    /// search.
    pub fn has_match(&self) -> bool {
        self.match_count > 0
    }

    /// Return the statistics gathered for this sink.
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    fn record_occurrences(
        &mut self,
        searcher: &Searcher,
        bytes: &[u8],
        range: std::ops::Range<usize>,
    ) -> io::Result<()> {
        self.json.occurrences.clear();
        let caps = self.scratch.captures(&self.matcher)?;
        capture_matches_in_context(
            searcher,
            &self.matcher,
            bytes,
            range.clone(),
            caps,
            &mut self.json.occurrences,
        )?;
        if !self.json.occurrences.is_empty()
            && self.json.occurrences.last().unwrap().overall().is_empty()
            && self.json.occurrences.last().unwrap().overall().start()
                >= range.end
        {
            self.json.occurrences.pop().unwrap();
        }
        Ok(())
    }

    fn write_begin_message(&mut self) -> io::Result<()> {
        if self.begin_printed {
            return Ok(());
        }
        let msg =
            jsont::CaptureMessage::Begin(jsont::Begin { path: self.path });
        self.json.write_message(&msg)?;
        self.begin_printed = true;
        Ok(())
    }

    fn occurrences<'a>(
        &self,
        bytes: &'a [u8],
        absolute_offset: u64,
    ) -> Vec<jsont::CaptureOccurrence<'a>> {
        self.json
            .occurrences
            .iter()
            .map(|occurrence| {
                let captures = occurrence
                    .captures()
                    .iter()
                    .enumerate()
                    .map(|(index, capture)| match *capture {
                        Some(span) => jsont::CaptureGroup {
                            index,
                            name: self
                                .matcher
                                .capture_name(index)
                                .map(str::to_string),
                            m: Some(&bytes[span]),
                            start: Some(span.start()),
                            end: Some(span.end()),
                            absolute_start: Some(
                                absolute_offset + span.start() as u64,
                            ),
                            absolute_end: Some(
                                absolute_offset + span.end() as u64,
                            ),
                        },
                        None => jsont::CaptureGroup {
                            index,
                            name: self
                                .matcher
                                .capture_name(index)
                                .map(str::to_string),
                            m: None,
                            start: None,
                            end: None,
                            absolute_start: None,
                            absolute_end: None,
                        },
                    })
                    .collect::<Vec<_>>();
                let overall = occurrence.overall();
                jsont::CaptureOccurrence {
                    m: &bytes[overall],
                    start: overall.start(),
                    end: overall.end(),
                    absolute_start: absolute_offset + overall.start() as u64,
                    absolute_end: absolute_offset + overall.end() as u64,
                    captures,
                }
            })
            .collect::<Vec<_>>()
    }
}

impl<'p, 's, M: Matcher, W: io::Write> Sink
    for JSONCapturesSink<'p, 's, M, W>
{
    type Error = io::Error;

    fn matched(
        &mut self,
        searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, io::Error> {
        self.match_count += 1;
        self.write_begin_message()?;
        self.record_occurrences(
            searcher,
            mat.buffer(),
            mat.bytes_range_in_buffer(),
        )?;
        self.stats.add_matches(self.json.occurrences.len() as u64);
        self.stats.add_matched_lines(mat.lines().count() as u64);

        let occurrences =
            self.occurrences(mat.bytes(), mat.absolute_byte_offset());
        let msg = jsont::CaptureMessage::Match(jsont::CaptureMatch {
            path: self.path,
            lines: self.json.config.lines.then_some(mat.bytes()),
            line_number: mat.line_number(),
            absolute_offset: mat.absolute_byte_offset(),
            occurrences,
        });
        self.json.write_message(&msg)?;
        Ok(true)
    }

    fn context(
        &mut self,
        searcher: &Searcher,
        ctx: &SinkContext<'_>,
    ) -> Result<bool, io::Error> {
        self.write_begin_message()?;
        self.json.occurrences.clear();

        if searcher.invert_match() {
            self.record_occurrences(
                searcher,
                ctx.bytes(),
                0..ctx.bytes().len(),
            )?;
        }
        let occurrences =
            self.occurrences(ctx.bytes(), ctx.absolute_byte_offset());
        let msg = jsont::CaptureMessage::Context(jsont::CaptureContext {
            path: self.path,
            lines: self.json.config.lines.then_some(ctx.bytes()),
            line_number: ctx.line_number(),
            absolute_offset: ctx.absolute_byte_offset(),
            occurrences,
        });
        self.json.write_message(&msg)?;
        Ok(true)
    }

    fn binary_data(
        &mut self,
        _searcher: &Searcher,
        _binary_byte_offset: u64,
    ) -> Result<bool, io::Error> {
        Ok(true)
    }

    fn begin(&mut self, _searcher: &Searcher) -> Result<bool, io::Error> {
        self.json.wtr.reset_count();
        self.start_time = Instant::now();
        self.match_count = 0;
        self.binary_byte_offset = None;

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
        self.binary_byte_offset = finish.binary_byte_offset();
        self.stats.add_elapsed(self.start_time.elapsed());
        self.stats.add_searches(1);
        if self.match_count > 0 {
            self.stats.add_searches_with_match(1);
        }
        self.stats.add_bytes_searched(finish.byte_count());
        self.stats.add_bytes_printed(self.json.wtr.count());

        if !self.begin_printed {
            return Ok(());
        }
        let msg = jsont::CaptureMessage::End(jsont::End {
            path: self.path,
            binary_offset: finish.binary_byte_offset(),
            stats: self.stats.clone(),
        });
        self.json.write_message(&msg)?;
        Ok(())
    }
}
