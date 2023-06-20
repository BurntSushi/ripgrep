use {
    grep_matcher::LineTerminator,
    regex_syntax::hir::{self, Hir, HirKind},
};

use crate::error::{Error, ErrorKind};

/// Return an HIR that is guaranteed to never match the given line terminator,
/// if possible.
///
/// If the transformation isn't possible, then an error is returned.
///
/// In general, if a literal line terminator occurs anywhere in the HIR, then
/// this will return an error. However, if the line terminator occurs within
/// a character class with at least one other character (that isn't also a line
/// terminator), then the line terminator is simply stripped from that class.
///
/// If the given line terminator is not ASCII, then this function returns an
/// error.
///
/// Note that as of regex 1.9, this routine could theoretically be implemented
/// without returning an error. Namely, for example, we could turn
/// `foo\nbar` into `foo[a&&b]bar`. That is, replace line terminators with a
/// sub-expression that can never match anything. Thus, ripgrep would accept
/// such regexes and just silently not match anything. Regex versions prior to 1.8
/// don't support such constructs. I ended up deciding to leave the existing
/// behavior of returning an error instead. For example:
///
/// ```text
/// $ echo -n 'foo\nbar\n' | rg 'foo\nbar'
/// the literal '"\n"' is not allowed in a regex
///
/// Consider enabling multiline mode with the --multiline flag (or -U for short).
/// When multiline mode is enabled, new line characters can be matched.
/// ```
///
/// This looks like a good error message to me, and even suggests a flag that
/// the user can use instead.
pub(crate) fn strip_from_match(
    expr: Hir,
    line_term: LineTerminator,
) -> Result<Hir, Error> {
    if line_term.is_crlf() {
        let expr1 = strip_from_match_ascii(expr, b'\r')?;
        strip_from_match_ascii(expr1, b'\n')
    } else {
        strip_from_match_ascii(expr, line_term.as_byte())
    }
}

/// The implementation of strip_from_match. The given byte must be ASCII.
/// This function returns an error otherwise. It also returns an error if
/// it couldn't remove `\n` from the given regex without leaving an empty
/// character class in its place.
fn strip_from_match_ascii(expr: Hir, byte: u8) -> Result<Hir, Error> {
    if !byte.is_ascii() {
        return Err(Error::new(ErrorKind::InvalidLineTerminator(byte)));
    }
    let ch = char::from(byte);
    let invalid = || Err(Error::new(ErrorKind::NotAllowed(ch.to_string())));
    Ok(match expr.into_kind() {
        HirKind::Empty => Hir::empty(),
        HirKind::Literal(hir::Literal(lit)) => {
            if lit.iter().find(|&&b| b == byte).is_some() {
                return invalid();
            }
            Hir::literal(lit)
        }
        HirKind::Class(hir::Class::Unicode(mut cls)) => {
            if cls.ranges().is_empty() {
                return Ok(Hir::class(hir::Class::Unicode(cls)));
            }
            let remove = hir::ClassUnicode::new(Some(
                hir::ClassUnicodeRange::new(ch, ch),
            ));
            cls.difference(&remove);
            if cls.ranges().is_empty() {
                return invalid();
            }
            Hir::class(hir::Class::Unicode(cls))
        }
        HirKind::Class(hir::Class::Bytes(mut cls)) => {
            if cls.ranges().is_empty() {
                return Ok(Hir::class(hir::Class::Bytes(cls)));
            }
            let remove = hir::ClassBytes::new(Some(
                hir::ClassBytesRange::new(byte, byte),
            ));
            cls.difference(&remove);
            if cls.ranges().is_empty() {
                return invalid();
            }
            Hir::class(hir::Class::Bytes(cls))
        }
        HirKind::Look(x) => Hir::look(x),
        HirKind::Repetition(mut x) => {
            x.sub = Box::new(strip_from_match_ascii(*x.sub, byte)?);
            Hir::repetition(x)
        }
        HirKind::Capture(mut x) => {
            x.sub = Box::new(strip_from_match_ascii(*x.sub, byte)?);
            Hir::capture(x)
        }
        HirKind::Concat(xs) => {
            let xs = xs
                .into_iter()
                .map(|e| strip_from_match_ascii(e, byte))
                .collect::<Result<Vec<Hir>, Error>>()?;
            Hir::concat(xs)
        }
        HirKind::Alternation(xs) => {
            let xs = xs
                .into_iter()
                .map(|e| strip_from_match_ascii(e, byte))
                .collect::<Result<Vec<Hir>, Error>>()?;
            Hir::alternation(xs)
        }
    })
}

#[cfg(test)]
mod tests {
    use regex_syntax::Parser;

    use super::{strip_from_match, LineTerminator};
    use crate::error::Error;

    fn roundtrip(pattern: &str, byte: u8) -> String {
        roundtrip_line_term(pattern, LineTerminator::byte(byte)).unwrap()
    }

    fn roundtrip_crlf(pattern: &str) -> String {
        roundtrip_line_term(pattern, LineTerminator::crlf()).unwrap()
    }

    fn roundtrip_err(pattern: &str, byte: u8) -> Result<String, Error> {
        roundtrip_line_term(pattern, LineTerminator::byte(byte))
    }

    fn roundtrip_line_term(
        pattern: &str,
        line_term: LineTerminator,
    ) -> Result<String, Error> {
        let expr1 = Parser::new().parse(pattern).unwrap();
        let expr2 = strip_from_match(expr1, line_term)?;
        Ok(expr2.to_string())
    }

    #[test]
    fn various() {
        assert_eq!(roundtrip(r"[a\n]", b'\n'), "a");
        assert_eq!(roundtrip(r"[a\n]", b'a'), "\n");
        assert_eq!(roundtrip_crlf(r"[a\n]"), "a");
        assert_eq!(roundtrip_crlf(r"[a\r]"), "a");
        assert_eq!(roundtrip_crlf(r"[a\r\n]"), "a");

        assert_eq!(roundtrip(r"(?-u)\s", b'a'), r"(?-u:[\x09-\x0D\x20])");
        assert_eq!(roundtrip(r"(?-u)\s", b'\n'), r"(?-u:[\x09\x0B-\x0D\x20])");

        assert!(roundtrip_err(r"\n", b'\n').is_err());
        assert!(roundtrip_err(r"abc\n", b'\n').is_err());
        assert!(roundtrip_err(r"\nabc", b'\n').is_err());
        assert!(roundtrip_err(r"abc\nxyz", b'\n').is_err());
        assert!(roundtrip_err(r"\x0A", b'\n').is_err());
        assert!(roundtrip_err(r"\u000A", b'\n').is_err());
        assert!(roundtrip_err(r"\U0000000A", b'\n').is_err());
        assert!(roundtrip_err(r"\u{A}", b'\n').is_err());
        assert!(roundtrip_err("\n", b'\n').is_err());
    }
}
