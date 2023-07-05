/*!
ripgrep, as a library.

This library is intended to provide a high level facade to the crates that
make up ripgrep's core searching routines. However, there is no high level
documentation available yet guiding users on how to fit all of the pieces
together.

Every public API item in the constituent crates is documented, but examples
are sparse.

A cookbook and a guide are planned.
*/

pub extern crate grep_cli as cli;
pub extern crate grep_matcher as matcher;
#[cfg(feature = "pcre2")]
pub extern crate grep_pcre2 as pcre2;
pub extern crate grep_printer as printer;
pub extern crate grep_regex as regex;
pub extern crate grep_searcher as searcher;
