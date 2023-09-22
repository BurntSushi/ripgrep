/*!
This crate provides featureful and fast printers that interoperate with the
[`grep-searcher`](https://docs.rs/grep-searcher)
crate.

# Brief overview

The [`Standard`](struct.Standard.html) printer shows results in a human
readable format, and is modeled after the formats used by standard grep-like
tools. Features include, but are not limited to, cross platform terminal
coloring, search & replace, multi-line result handling and reporting summary
statistics.

The [`JSON`](struct.JSON.html) printer shows results in a machine readable
format. To facilitate a stream of search results, the format uses
[JSON Lines](https://jsonlines.org/)
by emitting a series of messages as search results are found.

The [`Summary`](struct.Summary.html) printer shows *aggregate* results for a
single search in a human readable format, and is modeled after similar formats
found in standard grep-like tools. This printer is useful for showing the total
number of matches and/or printing file paths that either contain or don't
contain matches.

# Example

This example shows how to create a "standard" printer and execute a search.

```
use {
    grep_regex::RegexMatcher,
    grep_printer::Standard,
    grep_searcher::Searcher,
};

const SHERLOCK: &'static [u8] = b"\
For the Doctor Watsons of this world, as opposed to the Sherlock
Holmeses, success in the province of detective work must always
be, to a very large extent, the result of luck. Sherlock Holmes
can extract a clew from a wisp of straw or a flake of cigar ash;
but Doctor Watson has to have it taken out for him and dusted,
and exhibited clearly, with a label attached.
";

let matcher = RegexMatcher::new(r"Sherlock")?;
let mut printer = Standard::new_no_color(vec![]);
Searcher::new().search_slice(&matcher, SHERLOCK, printer.sink(&matcher))?;

// into_inner gives us back the underlying writer we provided to
// new_no_color, which is wrapped in a termcolor::NoColor. Thus, a second
// into_inner gives us back the actual buffer.
let output = String::from_utf8(printer.into_inner().into_inner())?;
let expected = "\
1:For the Doctor Watsons of this world, as opposed to the Sherlock
3:be, to a very large extent, the result of luck. Sherlock Holmes
";
assert_eq!(output, expected);
# Ok::<(), Box<dyn std::error::Error>>(())
```
*/

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub use crate::{
    color::{default_color_specs, ColorError, ColorSpecs, UserColorSpec},
    hyperlink::{
        HyperlinkConfig, HyperlinkEnvironment, HyperlinkFormat,
        HyperlinkFormatError,
    },
    path::{PathPrinter, PathPrinterBuilder},
    standard::{Standard, StandardBuilder, StandardSink},
    stats::Stats,
    summary::{Summary, SummaryBuilder, SummaryKind, SummarySink},
};

#[cfg(feature = "serde")]
pub use crate::json::{JSONBuilder, JSONSink, JSON};

// The maximum number of bytes to execute a search to account for look-ahead.
//
// This is an unfortunate kludge since PCRE2 doesn't provide a way to search
// a substring of some input while accounting for look-ahead. In theory, we
// could refactor the various 'grep' interfaces to account for it, but it would
// be a large change. So for now, we just let PCRE2 go looking a bit for a
// match without searching the entire rest of the contents.
//
// Note that this kludge is only active in multi-line mode.
const MAX_LOOK_AHEAD: usize = 128;

#[macro_use]
mod macros;

mod color;
mod counter;
mod hyperlink;
mod hyperlink_aliases;
#[cfg(feature = "serde")]
mod json;
#[cfg(feature = "serde")]
mod jsont;
mod path;
mod standard;
mod stats;
mod summary;
mod util;
