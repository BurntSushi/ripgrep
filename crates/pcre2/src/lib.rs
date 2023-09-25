/*!
An implementation of `grep-matcher`'s `Matcher` trait for
[PCRE2](https://www.pcre.org/).
*/

#![deny(missing_docs)]

pub use pcre2::{is_jit_available, version};

pub use crate::{
    error::{Error, ErrorKind},
    matcher::{RegexCaptures, RegexMatcher, RegexMatcherBuilder},
};

mod error;
mod matcher;
