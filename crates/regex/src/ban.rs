use regex_syntax::hir::{
    self, ClassBytesRange, ClassUnicodeRange, Hir, HirKind,
};

use crate::error::{Error, ErrorKind};

/// Returns an error when a sub-expression in `expr` must match `byte`.
pub(crate) fn check(expr: &Hir, byte: u8) -> Result<(), Error> {
    assert!(byte.is_ascii(), "ban byte must be ASCII");
    let ch = char::from(byte);
    let invalid = || Err(Error::new(ErrorKind::Banned(byte)));
    match expr.kind() {
        HirKind::Empty => {}
        HirKind::Literal(hir::Literal(ref lit)) => {
            if lit.iter().find(|&&b| b == byte).is_some() {
                return invalid();
            }
        }
        HirKind::Class(hir::Class::Unicode(ref cls)) => {
            if cls.ranges().iter().map(|r| r.len()).sum::<usize>() == 1 {
                let contains =
                    |r: &&ClassUnicodeRange| r.start() <= ch && ch <= r.end();
                if cls.ranges().iter().find(contains).is_some() {
                    return invalid();
                }
            }
        }
        HirKind::Class(hir::Class::Bytes(ref cls)) => {
            if cls.ranges().iter().map(|r| r.len()).sum::<usize>() == 1 {
                let contains = |r: &&ClassBytesRange| {
                    r.start() <= byte && byte <= r.end()
                };
                if cls.ranges().iter().find(contains).is_some() {
                    return invalid();
                }
            }
        }
        HirKind::Look(_) => {}
        HirKind::Repetition(ref x) => check(&x.sub, byte)?,
        HirKind::Capture(ref x) => check(&x.sub, byte)?,
        HirKind::Concat(ref xs) => {
            for x in xs.iter() {
                check(x, byte)?;
            }
        }
        HirKind::Alternation(ref xs) => {
            for x in xs.iter() {
                check(x, byte)?;
            }
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use regex_syntax::Parser;

    /// Returns true when the given pattern is detected to contain the given
    /// banned byte.
    fn check(pattern: &str, byte: u8) -> bool {
        let hir = Parser::new().parse(pattern).unwrap();
        super::check(&hir, byte).is_err()
    }

    #[test]
    fn various() {
        assert!(check(r"\x00", 0));
        assert!(check(r"a\x00", 0));
        assert!(check(r"\x00b", 0));
        assert!(check(r"a\x00b", 0));
        assert!(check(r"\x00|ab", 0));
        assert!(check(r"ab|\x00", 0));
        assert!(check(r"\x00?", 0));
        assert!(check(r"(\x00)", 0));

        assert!(check(r"[\x00]", 0));
        assert!(check(r"[^[^\x00]]", 0));

        assert!(!check(r"[^\x00]", 0));
        assert!(!check(r"[\x00a]", 0));
    }
}
