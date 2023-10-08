use memchr::memchr;

/// Interpolate capture references in `replacement` and write the interpolation
/// result to `dst`. References in `replacement` take the form of $N or $name,
/// where `N` is a capture group index and `name` is a capture group name. The
/// function provided, `name_to_index`, maps capture group names to indices.
///
/// The `append` function given is responsible for writing the replacement
/// to the `dst` buffer. That is, it is called with the capture group index
/// of a capture group reference and is expected to resolve the index to its
/// corresponding matched text. If no such match exists, then `append` should
/// not write anything to its given buffer.
#[inline]
pub fn interpolate<A, N>(
    mut replacement: &[u8],
    mut append: A,
    mut name_to_index: N,
    dst: &mut Vec<u8>,
) where
    A: FnMut(usize, &mut Vec<u8>),
    N: FnMut(&str) -> Option<usize>,
{
    while !replacement.is_empty() {
        match memchr(b'$', replacement) {
            None => break,
            Some(i) => {
                dst.extend(&replacement[..i]);
                replacement = &replacement[i..];
            }
        }
        if replacement.get(1).map_or(false, |&b| b == b'$') {
            dst.push(b'$');
            replacement = &replacement[2..];
            continue;
        }
        debug_assert!(!replacement.is_empty());
        let cap_ref = match find_cap_ref(replacement) {
            Some(cap_ref) => cap_ref,
            None => {
                dst.push(b'$');
                replacement = &replacement[1..];
                continue;
            }
        };
        replacement = &replacement[cap_ref.end..];
        match cap_ref.cap {
            Ref::Number(i) => append(i, dst),
            Ref::Named(name) => {
                if let Some(i) = name_to_index(name) {
                    append(i, dst);
                }
            }
        }
    }
    dst.extend(replacement);
}

/// `CaptureRef` represents a reference to a capture group inside some text.
/// The reference is either a capture group name or a number.
///
/// It is also tagged with the position in the text immediately proceeding the
/// capture reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CaptureRef<'a> {
    cap: Ref<'a>,
    end: usize,
}

/// A reference to a capture group in some text.
///
/// e.g., `$2`, `$foo`, `${foo}`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ref<'a> {
    Named(&'a str),
    Number(usize),
}

impl<'a> From<&'a str> for Ref<'a> {
    #[inline]
    fn from(x: &'a str) -> Ref<'a> {
        Ref::Named(x)
    }
}

impl From<usize> for Ref<'static> {
    #[inline]
    fn from(x: usize) -> Ref<'static> {
        Ref::Number(x)
    }
}

/// Parses a possible reference to a capture group name in the given text,
/// starting at the beginning of `replacement`.
///
/// If no such valid reference could be found, None is returned.
#[inline]
fn find_cap_ref(replacement: &[u8]) -> Option<CaptureRef<'_>> {
    let mut i = 0;
    if replacement.len() <= 1 || replacement[0] != b'$' {
        return None;
    }
    let mut brace = false;
    i += 1;
    if replacement[i] == b'{' {
        brace = true;
        i += 1;
    }
    let mut cap_end = i;
    while replacement.get(cap_end).map_or(false, is_valid_cap_letter) {
        cap_end += 1;
    }
    if cap_end == i {
        return None;
    }
    // We just verified that the range 0..cap_end is valid ASCII, so it must
    // therefore be valid UTF-8. If we really cared, we could avoid this UTF-8
    // check with an unchecked conversion or by parsing the number straight
    // from &[u8].
    let cap = std::str::from_utf8(&replacement[i..cap_end])
        .expect("valid UTF-8 capture name");
    if brace {
        if !replacement.get(cap_end).map_or(false, |&b| b == b'}') {
            return None;
        }
        cap_end += 1;
    }
    Some(CaptureRef {
        cap: match cap.parse::<u32>() {
            Ok(i) => Ref::Number(i as usize),
            Err(_) => Ref::Named(cap),
        },
        end: cap_end,
    })
}

/// Returns true if and only if the given byte is allowed in a capture name.
#[inline]
fn is_valid_cap_letter(b: &u8) -> bool {
    match *b {
        b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{find_cap_ref, interpolate, CaptureRef};

    macro_rules! find {
        ($name:ident, $text:expr) => {
            #[test]
            fn $name() {
                assert_eq!(None, find_cap_ref($text.as_bytes()));
            }
        };
        ($name:ident, $text:expr, $capref:expr) => {
            #[test]
            fn $name() {
                assert_eq!(Some($capref), find_cap_ref($text.as_bytes()));
            }
        };
    }

    macro_rules! c {
        ($name_or_number:expr, $pos:expr) => {
            CaptureRef { cap: $name_or_number.into(), end: $pos }
        };
    }

    find!(find_cap_ref1, "$foo", c!("foo", 4));
    find!(find_cap_ref2, "${foo}", c!("foo", 6));
    find!(find_cap_ref3, "$0", c!(0, 2));
    find!(find_cap_ref4, "$5", c!(5, 2));
    find!(find_cap_ref5, "$10", c!(10, 3));
    find!(find_cap_ref6, "$42a", c!("42a", 4));
    find!(find_cap_ref7, "${42}a", c!(42, 5));
    find!(find_cap_ref8, "${42");
    find!(find_cap_ref9, "${42 ");
    find!(find_cap_ref10, " $0 ");
    find!(find_cap_ref11, "$");
    find!(find_cap_ref12, " ");
    find!(find_cap_ref13, "");

    // A convenience routine for using interpolate's unwieldy but flexible API.
    fn interpolate_string(
        mut name_to_index: Vec<(&'static str, usize)>,
        caps: Vec<&'static str>,
        replacement: &str,
    ) -> String {
        name_to_index.sort_by_key(|x| x.0);

        let mut dst = vec![];
        interpolate(
            replacement.as_bytes(),
            |i, dst| {
                if let Some(&s) = caps.get(i) {
                    dst.extend(s.as_bytes());
                }
            },
            |name| -> Option<usize> {
                name_to_index
                    .binary_search_by_key(&name, |x| x.0)
                    .ok()
                    .map(|i| name_to_index[i].1)
            },
            &mut dst,
        );
        String::from_utf8(dst).unwrap()
    }

    macro_rules! interp {
        ($name:ident, $map:expr, $caps:expr, $hay:expr, $expected:expr $(,)*) => {
            #[test]
            fn $name() {
                assert_eq!($expected, interpolate_string($map, $caps, $hay));
            }
        };
    }

    interp!(
        interp1,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test $foo test",
        "test xxx test",
    );

    interp!(
        interp2,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test$footest",
        "test",
    );

    interp!(
        interp3,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test${foo}test",
        "testxxxtest",
    );

    interp!(
        interp4,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test$2test",
        "test",
    );

    interp!(
        interp5,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test${2}test",
        "testxxxtest",
    );

    interp!(
        interp6,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test $$foo test",
        "test $foo test",
    );

    interp!(
        interp7,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test $foo",
        "test xxx",
    );

    interp!(
        interp8,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "$foo test",
        "xxx test",
    );

    interp!(
        interp9,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test $bar$foo",
        "test yyyxxx",
    );

    interp!(
        interp10,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test $ test",
        "test $ test",
    );

    interp!(
        interp11,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${} test",
        "test ${} test",
    );

    interp!(
        interp12,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${ } test",
        "test ${ } test",
    );

    interp!(
        interp13,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${a b} test",
        "test ${a b} test",
    );

    interp!(
        interp14,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${a} test",
        "test  test",
    );
}
