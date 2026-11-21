// Macros useful for testing.
#[macro_use]
mod macros;

// Corpora.
mod hay;
// Utilities for making tests nicer to read and easier to write.
mod util;

// Tests for ripgrep's handling of binary files.
mod binary;
// Tests related to most features in ripgrep. If you're adding something new
// to ripgrep, tests should probably go in here.
mod feature;
// Tests for ripgrep's JSON format.
mod json;
// Miscellaneous tests grouped in a haphazard manner. Try not to add more.
mod misc;
// Tests for ripgrep's multiline search support.
mod multiline;
// Regression tests.
mod regression;
