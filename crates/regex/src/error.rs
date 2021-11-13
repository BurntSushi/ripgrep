use std::error;
use std::fmt;

use crate::util;

/// An error that can occur in this crate.
///
/// Generally, this error corresponds to problems building a regular
/// expression, whether it's in parsing, compilation or a problem with
/// guaranteeing a configured optimization.
#[derive(Clone, Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Error {
        Error { kind }
    }

    pub(crate) fn regex<E: error::Error>(err: E) -> Error {
        Error { kind: ErrorKind::Regex(err.to_string()) }
    }

    /// Return the kind of this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

/// The kind of an error that can occur.
#[derive(Clone, Debug)]
pub enum ErrorKind {
    /// An error that occurred as a result of parsing a regular expression.
    /// This can be a syntax error or an error that results from attempting to
    /// compile a regular expression that is too big.
    ///
    /// The string here is the underlying error converted to a string.
    Regex(String),
    /// An error that occurs when a building a regex that isn't permitted to
    /// match a line terminator. In general, building the regex will do its
    /// best to make matching a line terminator impossible (e.g., by removing
    /// `\n` from the `\s` character class), but if the regex contains a
    /// `\n` literal, then there is no reasonable choice that can be made and
    /// therefore an error is reported.
    ///
    /// The string is the literal sequence found in the regex that is not
    /// allowed.
    NotAllowed(String),
    /// This error occurs when a non-ASCII line terminator was provided.
    ///
    /// The invalid byte is included in this error.
    InvalidLineTerminator(u8),
    /// Hints that destructuring should not be exhaustive.
    ///
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::Regex(_) => "regex error",
            ErrorKind::NotAllowed(_) => "literal not allowed",
            ErrorKind::InvalidLineTerminator(_) => "invalid line terminator",
            ErrorKind::__Nonexhaustive => unreachable!(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::Regex(ref s) => write!(f, "{}", s),
            ErrorKind::NotAllowed(ref lit) => {
                write!(f, "the literal '{:?}' is not allowed in a regex", lit)
            }
            ErrorKind::InvalidLineTerminator(byte) => {
                let x = util::show_bytes(&[byte]);
                write!(f, "line terminators must be ASCII, but '{}' is not", x)
            }
            ErrorKind::__Nonexhaustive => unreachable!(),
        }
    }
}
