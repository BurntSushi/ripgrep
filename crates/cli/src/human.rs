use std::error;
use std::fmt;
use std::io;
use std::num::ParseIntError;

use regex::Regex;

/// An error that occurs when parsing a human readable size description.
///
/// This error provides an end user friendly message describing why the
/// description coudln't be parsed and what the expected format is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseSizeError {
    original: String,
    kind: ParseSizeErrorKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParseSizeErrorKind {
    InvalidFormat,
    InvalidInt(ParseIntError),
    Overflow,
}

impl ParseSizeError {
    fn format(original: &str) -> ParseSizeError {
        ParseSizeError {
            original: original.to_string(),
            kind: ParseSizeErrorKind::InvalidFormat,
        }
    }

    fn int(original: &str, err: ParseIntError) -> ParseSizeError {
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

impl error::Error for ParseSizeError {
    fn description(&self) -> &str {
        "invalid size"
    }
}

impl fmt::Display for ParseSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ParseSizeErrorKind::*;

        match self.kind {
            InvalidFormat => write!(
                f,
                "invalid format for size '{}', which should be a sequence \
                     of digits followed by an optional 'K', 'M' or 'G' \
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

impl From<ParseSizeError> for io::Error {
    fn from(size_err: ParseSizeError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, size_err)
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
    lazy_static::lazy_static! {
        // Normally I'd just parse something this simple by hand to avoid the
        // regex dep, but we bring regex in any way for glob matching, so might
        // as well use it.
        static ref RE: Regex = Regex::new(r"^([0-9]+)([KMG])?$").unwrap();
    }

    let caps = match RE.captures(size) {
        Some(caps) => caps,
        None => return Err(ParseSizeError::format(size)),
    };
    let value: u64 =
        caps[1].parse().map_err(|err| ParseSizeError::int(size, err))?;
    let suffix = match caps.get(2) {
        None => return Ok(value),
        Some(cap) => cap.as_str(),
    };
    let bytes = match suffix {
        "K" => value.checked_mul(1 << 10),
        "M" => value.checked_mul(1 << 20),
        "G" => value.checked_mul(1 << 30),
        // Because if the regex matches this group, it must be [KMG].
        _ => unreachable!(),
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
