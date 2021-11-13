/*!
An implementation of `grep-matcher`'s `Matcher` trait for
[PCRE2](https://www.pcre.org/).
*/

#![deny(missing_docs)]

pub use crate::error::{Error, ErrorKind};
pub use crate::matcher::{RegexCaptures, RegexMatcher, RegexMatcherBuilder};
pub use pcre2::{is_jit_available, version};

mod error;
mod matcher;
