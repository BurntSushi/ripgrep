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
    pub(crate) fn regex<E: std::error::Error>(err: E) -> Error {
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
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::Regex(_) => "regex error",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ErrorKind::Regex(ref s) => write!(f, "{}", s),
        }
    }
}
