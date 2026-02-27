/// An error that occurs when parsing a human readable size description.
///
/// This error provides an end user friendly message describing why the
/// description couldn't be parsed and what the expected format is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseSizeError {
    original: String,
    kind: ParseSizeErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParseSizeErrorKind {
    InvalidFormat,
    InvalidInt(std::num::ParseIntError),
    Overflow,
}

impl ParseSizeError {
    fn format(original: &str) -> ParseSizeError {
        ParseSizeError {
            original: original.to_string(),
            kind: ParseSizeErrorKind::InvalidFormat,
        }
    }

    fn int(original: &str, err: std::num::ParseIntError) -> ParseSizeError {
        ParseSizeError {
            original: original.to_string(),
            kind: ParseSizeErrorKind::InvalidInt(err),
        }
    }

    fn overflow(original: &str) -> ParseSizeError {
        ParseSizeError {
            original: original.to_string(),
            kind: ParseSizeErrorKind::Overflow,
        }
    }
}

impl std::error::Error for ParseSizeError {}

impl std::fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use self::ParseSizeErrorKind::*;

        match self.kind {
            InvalidFormat => write!(
                f,
                "invalid format for size '{}', which should be a non-empty \
                 sequence of digits followed by an optional 'K', 'M' or 'G' \
                 suffix",
                self.original
            ),
            InvalidInt(ref err) => write!(
                f,
                "invalid integer found in size '{}': {}",
                self.original, err
            ),
            Overflow => write!(f, "size too big in '{}'", self.original),
        }
    }
}

impl From<ParseSizeError> for std::io::Error {
    fn from(size_err: ParseSizeError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, size_err)
    }
}

/// Parse a human readable size like `2M` into a corresponding number of bytes.
///
/// Supported size suffixes are `K` (for kilobyte), `M` (for megabyte) and `G`
/// (for gigabyte). If a size suffix is missing, then the size is interpreted
/// as bytes. If the size is too big to fit into a `u64`, then this returns an
/// error.
///
/// Additional suffixes may be added over time.
pub fn parse_human_readable_size(size: &str) -> Result<u64, ParseSizeError> {
    let digits_end =
        size.as_bytes().iter().take_while(|&b| b.is_ascii_digit()).count();
    let digits = &size[..digits_end];
    if digits.is_empty() {
        return Err(ParseSizeError::format(size));
    }
    let value =
        digits.parse::<u64>().map_err(|e| ParseSizeError::int(size, e))?;

    let suffix = &size[digits_end..];
    if suffix.is_empty() {
        return Ok(value);
    }
    let bytes = match suffix {
        "K" => value.checked_mul(1 << 10),
        "M" => value.checked_mul(1 << 20),
        "G" => value.checked_mul(1 << 30),
        _ => return Err(ParseSizeError::format(size)),
    };
    bytes.ok_or_else(|| ParseSizeError::overflow(size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_none() {
        let x = parse_human_readable_size("123").unwrap();
        assert_eq!(123, x);
    }

    #[test]
    fn suffix_k() {
        let x = parse_human_readable_size("123K").unwrap();
        assert_eq!(123 * (1 << 10), x);
    }

    #[test]
    fn suffix_m() {
        let x = parse_human_readable_size("123M").unwrap();
        assert_eq!(123 * (1 << 20), x);
    }

    #[test]
    fn suffix_g() {
        let x = parse_human_readable_size("123G").unwrap();
        assert_eq!(123 * (1 << 30), x);
    }

    #[test]
    fn invalid_empty() {
        assert!(parse_human_readable_size("").is_err());
    }

    #[test]
    fn invalid_non_digit() {
        assert!(parse_human_readable_size("a").is_err());
    }

    #[test]
    fn invalid_overflow() {
        assert!(parse_human_readable_size("9999999999999999G").is_err());
    }

    #[test]
    fn invalid_suffix() {
        assert!(parse_human_readable_size("123T").is_err());
    }
}
