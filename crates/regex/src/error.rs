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

    pub(crate) fn regex(err: regex_automata::meta::BuildError) -> Error {
        if let Some(size_limit) = err.size_limit() {
            let kind = ErrorKind::Regex(format!(
                "compiled regex exceeds size limit of {size_limit}",
            ));
            Error { kind }
        } else if let Some(ref err) = err.syntax_error() {
            Error::generic(err)
        } else {
            Error::generic(err)
        }
    }

    pub(crate) fn generic<E: std::error::Error>(err: E) -> Error {
        Error { kind: ErrorKind::Regex(err.to_string()) }
    }

    /// Return the kind of this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

/// The kind of an error that can occur.
#[derive(Clone, Debug)]
#[non_exhaustive]
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
    /// Occurs when a banned byte was found in a pattern.
    Banned(u8),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use bstr::ByteSlice;

        match self.kind {
            ErrorKind::Regex(ref s) => write!(f, "{}", s),
            ErrorKind::NotAllowed(ref lit) => {
                write!(f, "the literal {:?} is not allowed in a regex", lit)
            }
            ErrorKind::InvalidLineTerminator(byte) => {
                write!(
                    f,
                    "line terminators must be ASCII, but {byte:?} is not",
                    byte = [byte].as_bstr(),
                )
            }
            ErrorKind::Banned(byte) => {
                write!(
                    f,
                    "pattern contains {byte:?} but it is impossible to match",
                    byte = [byte].as_bstr(),
                )
            }
        }
    }
}
