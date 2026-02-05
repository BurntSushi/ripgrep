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
mod bridge_literals;
mod config;
mod error;
mod literal;
mod matcher;
mod non_matching;
mod strip;

pub use crate::bridge_literals::LiteralSequence;
