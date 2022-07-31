/// This module defines the types we use for Patch generation.

use std::io;

use bstr::ByteVec;
use grep_searcher::{SinkMatch, SinkContext};

/// The patch styles match different possible input types accepted by the
/// `patch` utiltiy.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PatchStyle {
    // The Unified format (originally GNU-only)
    Unified,
    /* TODO: determine if it's useful to support these formats
    Posix, // <- what should this be named? the 'classic' patch format
    Context,
    Ed,
    */
}

#[derive(Debug, Default)]
pub struct PatchHunk {
    // Only one starting line number is necessary, because multi-line replace is
    // not supported
    starting_line_number: Option<u64>,
    lines: Vec<PatchLine>,
}

/// A line in a hunk. Unfortunately, this must copy each line, since they cannot
/// be printed until the header has been printed, and matches provided to
/// `Sink.matched` are references that won't live that long.
#[derive(Debug)]
pub enum PatchLine {
    Unchanged(Vec<u8>),
    // Orig, new
    Changed(Vec<u8>, Vec<u8>)
}

impl PatchHunk {
    pub fn write<W: io::Write>(&self, wtr: &mut W, style: PatchStyle) -> Result<(), io::Error> {
        if style != PatchStyle::Unified {
            unimplemented!("only unified patch style supported for now");
        }
        match self.starting_line_number {
            Some(number) => 
                write!(
                    wtr, "@@ -{line},{count} +{line},{count} @@\n",
                    line=number, count=self.lines.len())?,
            // XXX change error type
            None => return Err(io::Error::new(io::ErrorKind::Other, "no line numbers")),
        }
        for line in &self.lines {
            line.write(&mut *wtr)?;
        }
        Ok(())
    }

    pub fn add_context(&mut self, ctx: &SinkContext<'_>) {
        self.lines.push(PatchLine::Unchanged(ctx.bytes().to_vec()));
    }

    pub fn add_match(&mut self, mat: &SinkMatch<'_>, replacement: &[u8]) {
        // XXX validate `unwrap` here - when would a match not have a line
        // number? (Presumably this case would not be supported by this printer)
        let _ = self.starting_line_number.get_or_insert_with(|| mat.line_number().unwrap());
        let orig = mat.bytes().to_vec();
        let mut modified = replacement.to_vec();
        // Unlike the match, the replacement does not include the line ending.
        // XXX find out if line-endings need to be consolidated
        modified.push_char('\n');
        self.lines.push(PatchLine::Changed(orig, modified));
    }
}

impl PatchLine {
    fn write<W: io::Write>(&self, wtr: &mut W) -> Result<(), io::Error> {
        use PatchLine::*;
        match self {
            Unchanged(line) => {
                wtr.write(b" ")?;
                // XXX figure out if lines will have newlines included
                // XXX figure out if 'write' is still safe here, with potentially long lines
                wtr.write(&line)?;
            }
            Changed(old, new) => {
                wtr.write(b"-")?;
                wtr.write(&old)?;
                wtr.write(b"+")?;
                wtr.write(&new)?;
            }
        }
        Ok(())
    }
}