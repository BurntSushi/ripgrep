use std::io::{self, Write};
use std::path::Path;

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
        Config { style: Normal, }
    }
}

#[derive(Clone, Debug)]
pub struct PatchBuilder {
    config: Config,
}

#[derive(Debug)]
pub struct Patch<W> {
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
    stats: Stats,
}

impl<W: io::Write> Patch<W> {
    pub fn new(wtr: W) -> Patch<W> {
        PatchBuilder::new().build(wtr)
    }

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


}