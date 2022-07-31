use std::cell::RefCell;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use grep_matcher::{Matcher, Match};
use termcolor::WriteColor;

use crate::Stats;
use crate::counter::CounterWriter;

#[derive(Debug, Clone)]
enum PatchStyle {
    Normal,
    Context,
    Ed,
}

#[derive(Debug, Clone)]
struct Config {
    style: PatchStyle,
}

impl Default for Config {
    fn default() -> Config {
        Config { style: PatchStyle::Normal, }
    }
}

#[derive(Clone, Debug)]
pub struct PatchBuilder {
    config: Config,
}

impl PatchBuilder {
    pub fn new() -> PatchBuilder {
        PatchBuilder { config: Config::default() }
    }

    pub fn build<W: WriteColor>(&self, wtr: W) -> Patch<W> {
        Patch {
            config: self.config.clone(),
            wtr: RefCell::new(CounterWriter::new(wtr)),
            matches: vec![],
        }
    }
}

#[derive(Debug)]
pub struct Patch<W> {
    config: Config,
    wtr: RefCell<CounterWriter<W>>,
    matches: Vec<Match>,
}

#[derive(Debug)]
pub struct PatchSink<'p, 's, M: Matcher, W> {
    matcher: M,
    patch: &'s mut Patch<W>,
    path: Option<&'p Path>,
    start_time: Instant,
    match_count: u64,
    after_context_remaining: u64,
    binary_byte_offset: Option<u64>,
    begin_printed: bool,
    // XXX replace with 'Stats' if appropriate
    has_match: bool,
}

impl<'p, 's, M: Matcher, W> PatchSink<'p, 's, M, W> {
    fn has_match(&self) -> bool {
        return self.has_match;
    }
}

impl<W: io::Write> Patch<W> {
    pub fn sink_with_path<'p, 's, M, P>(
        &'s mut self,
        matcher: M,
        path: &'p P,
    ) -> PatchSink<'p, 's, M, W>
    where
        M: Matcher,
        P: ?Sized + AsRef<Path>,
    {
        PatchSink {
            matcher: matcher,
            patch: self,
            path: Some(path.as_ref()),
            start_time: Instant::now(),
            match_count: 0,
            after_context_remaining: 0,
            binary_byte_offset: None,
            begin_printed: false,
            has_match: false,
        }
    }


}