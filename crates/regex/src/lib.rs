/*!
An implementation of `grep-matcher`'s `Matcher` trait for Rust's regex engine.
*/
#![deny(missing_docs)]

pub use crate::{
    error::{Error, ErrorKind},
    matcher::{RegexCaptures, RegexMatcher, RegexMatcherBuilder},
};

mod ast;
mod ban;
mod config;
mod error;
mod literal;
mod matcher;
mod non_matching;
mod strip;
