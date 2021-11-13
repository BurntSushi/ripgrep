/*!
This crate provides an implementation of line oriented search, with optional
support for multi-line search.

# Brief overview

The principle type in this crate is a
[`Searcher`](struct.Searcher.html),
which can be configured and built by a
[`SearcherBuilder`](struct.SearcherBuilder.html).
A `Searcher` is responsible for reading bytes from a source (e.g., a file),
executing a search of those bytes using a `Matcher` (e.g., a regex) and then
reporting the results of that search to a
[`Sink`](trait.Sink.html)
(e.g., stdout). The `Searcher` itself is principally responsible for managing
the consumption of bytes from a source and applying a `Matcher` over those
bytes in an efficient way. The `Searcher` is also responsible for inverting
a search, counting lines, reporting contextual lines, detecting binary data
and even deciding whether or not to use memory maps.

A `Matcher` (which is defined in the
[`grep-matcher`](https://crates.io/crates/grep-matcher)
crate) is a trait for describing the lowest levels of pattern search in a
generic way. The interface itself is very similar to the interface of a regular
expression. For example, the
[`grep-regex`](https://crates.io/crates/grep-regex)
crate provides an implementation of the `Matcher` trait using Rust's
[`regex`](https://crates.io/crates/regex)
crate.

Finally, a `Sink` describes how callers receive search results producer by a
`Searcher`. This includes routines that are called at the beginning and end of
a search, in addition to routines that are called when matching or contextual
lines are found by the `Searcher`. Implementations of `Sink` can be trivially
simple, or extraordinarily complex, such as the
`Standard` printer found in the
[`grep-printer`](https://crates.io/crates/grep-printer)
crate, which effectively implements grep-like output.
This crate also provides convenience `Sink` implementations in the
[`sinks`](sinks/index.html)
sub-module for easy searching with closures.

# Example

This example shows how to execute the searcher and read the search results
using the
[`UTF8`](sinks/struct.UTF8.html)
implementation of `Sink`.

```
use std::error::Error;

use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::Searcher;
use grep_searcher::sinks::UTF8;

const SHERLOCK: &'static [u8] = b"\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
can extract a clew from a wisp of straw or a flake of cigar ash;
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.
";

# fn main() { example().unwrap() }
fn example() -> Result<(), Box<Error>> {
    let matcher = RegexMatcher::new(r"Doctor \w+")?;
    let mut matches: Vec<(u64, String)> = vec![];
    Searcher::new().search_slice(&matcher, SHERLOCK, UTF8(|lnum, line| {
        // We are guaranteed to find a match, so the unwrap is OK.
        let mymatch = matcher.find(line.as_bytes())?.unwrap();
        matches.push((lnum, line[mymatch].to_string()));
        Ok(true)
    }))?;

    assert_eq!(matches.len(), 2);
    assert_eq!(
        matches[0],
        (1, "Doctor Watsons".to_string())
    );
    assert_eq!(
        matches[1],
        (5, "Doctor Watson".to_string())
    );
    Ok(())
}
```

See also `examples/search-stdin.rs` from the root of this crate's directory
to see a similar example that accepts a pattern on the command line and
searches stdin.
*/

#![deny(missing_docs)]

pub use crate::lines::{LineIter, LineStep};
pub use crate::searcher::{
    BinaryDetection, ConfigError, Encoding, MmapChoice, Searcher,
    SearcherBuilder,
};
pub use crate::sink::sinks;
pub use crate::sink::{
    Sink, SinkContext, SinkContextKind, SinkError, SinkFinish, SinkMatch,
};

#[macro_use]
mod macros;

mod line_buffer;
mod lines;
mod searcher;
mod sink;
#[cfg(test)]
mod testutil;
