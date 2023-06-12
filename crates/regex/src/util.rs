/// Converts an arbitrary sequence of bytes to a literal suitable for building
/// a regular expression.
#[allow(dead_code)]
pub fn bytes_to_regex(bs: &[u8]) -> String {
    use regex_syntax::is_meta_character;
    use std::fmt::Write;

    let mut s = String::with_capacity(bs.len());
    for &b in bs {
        if b <= 0x7F && !is_meta_character(b as char) {
            write!(s, r"{}", b as char).unwrap();
        } else {
            write!(s, r"\x{:02x}", b).unwrap();
        }
    }
    s
}

/// Converts arbitrary bytes to a nice string.
pub fn show_bytes(bs: &[u8]) -> String {
    use std::ascii::escape_default;
    use std::str;

    let mut nice = String::new();
    for &b in bs {
        let part: Vec<u8> = escape_default(b).collect();
        nice.push_str(str::from_utf8(&part).unwrap());
    }
    nice
}
