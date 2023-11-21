// This module defines the types we use for JSON serialization. We specifically
// omit deserialization, partially because there isn't a clear use case for
// them at this time, but also because deserialization will complicate things.
// Namely, the types below are designed in a way that permits JSON
// serialization with little or no allocation. Allocation is often quite
// convenient for deserialization however, so these types would become a bit
// more complex.

use std::{borrow::Cow, path::Path};

pub(crate) enum Message<'a> {
    Begin(Begin<'a>),
    End(End<'a>),
    Match(Match<'a>),
    Context(Context<'a>),
}

impl<'a> serde::Serialize for Message<'a> {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("Message", 2)?;
        match *self {
            Message::Begin(ref msg) => {
                state.serialize_field("type", &"begin")?;
                state.serialize_field("data", msg)?;
            }
            Message::End(ref msg) => {
                state.serialize_field("type", &"end")?;
                state.serialize_field("data", msg)?;
            }
            Message::Match(ref msg) => {
                state.serialize_field("type", &"match")?;
                state.serialize_field("data", msg)?;
            }
            Message::Context(ref msg) => {
                state.serialize_field("type", &"context")?;
                state.serialize_field("data", msg)?;
            }
        }
        state.end()
    }
}

pub(crate) struct Begin<'a> {
    pub(crate) path: Option<&'a Path>,
}

impl<'a> serde::Serialize for Begin<'a> {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("Begin", 1)?;
        state.serialize_field("path", &self.path.map(Data::from_path))?;
        state.end()
    }
}

pub(crate) struct End<'a> {
    pub(crate) path: Option<&'a Path>,
    pub(crate) binary_offset: Option<u64>,
    pub(crate) stats: crate::stats::Stats,
}

impl<'a> serde::Serialize for End<'a> {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("End", 3)?;
        state.serialize_field("path", &self.path.map(Data::from_path))?;
        state.serialize_field("binary_offset", &self.binary_offset)?;
        state.serialize_field("stats", &self.stats)?;
        state.end()
    }
}

pub(crate) struct Match<'a> {
    pub(crate) path: Option<&'a Path>,
    pub(crate) lines: &'a [u8],
    pub(crate) line_number: Option<u64>,
    pub(crate) absolute_offset: u64,
    pub(crate) submatches: &'a [SubMatch<'a>],
}

impl<'a> serde::Serialize for Match<'a> {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("Match", 5)?;
        state.serialize_field("path", &self.path.map(Data::from_path))?;
        state.serialize_field("lines", &Data::from_bytes(self.lines))?;
        state.serialize_field("line_number", &self.line_number)?;
        state.serialize_field("absolute_offset", &self.absolute_offset)?;
        state.serialize_field("submatches", &self.submatches)?;
        state.end()
    }
}

pub(crate) struct Context<'a> {
    pub(crate) path: Option<&'a Path>,
    pub(crate) lines: &'a [u8],
    pub(crate) line_number: Option<u64>,
    pub(crate) absolute_offset: u64,
    pub(crate) submatches: &'a [SubMatch<'a>],
}

impl<'a> serde::Serialize for Context<'a> {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("Context", 5)?;
        state.serialize_field("path", &self.path.map(Data::from_path))?;
        state.serialize_field("lines", &Data::from_bytes(self.lines))?;
        state.serialize_field("line_number", &self.line_number)?;
        state.serialize_field("absolute_offset", &self.absolute_offset)?;
        state.serialize_field("submatches", &self.submatches)?;
        state.end()
    }
}

pub(crate) struct SubMatch<'a> {
    pub(crate) m: &'a [u8],
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl<'a> serde::Serialize for SubMatch<'a> {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("SubMatch", 3)?;
        state.serialize_field("match", &Data::from_bytes(self.m))?;
        state.serialize_field("start", &self.start)?;
        state.serialize_field("end", &self.end)?;
        state.end()
    }
}

/// Data represents things that look like strings, but may actually not be
/// valid UTF-8. To handle this, `Data` is serialized as an object with one
/// of two keys: `text` (for valid UTF-8) or `bytes` (for invalid UTF-8).
///
/// The happy path is valid UTF-8, which streams right through as-is, since
/// it is natively supported by JSON. When invalid UTF-8 is found, then it is
/// represented as arbitrary bytes and base64 encoded.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum Data<'a> {
    Text { text: Cow<'a, str> },
    Bytes { bytes: &'a [u8] },
}

impl<'a> Data<'a> {
    fn from_bytes(bytes: &[u8]) -> Data<'_> {
        match std::str::from_utf8(bytes) {
            Ok(text) => Data::Text { text: Cow::Borrowed(text) },
            Err(_) => Data::Bytes { bytes },
        }
    }

    #[cfg(unix)]
    fn from_path(path: &Path) -> Data<'_> {
        use std::os::unix::ffi::OsStrExt;

        match path.to_str() {
            Some(text) => Data::Text { text: Cow::Borrowed(text) },
            None => Data::Bytes { bytes: path.as_os_str().as_bytes() },
        }
    }

    #[cfg(not(unix))]
    fn from_path(path: &Path) -> Data {
        // Using lossy conversion means some paths won't round trip precisely,
        // but it's not clear what we should actually do. Serde rejects
        // non-UTF-8 paths, and OsStr's are serialized as a sequence of UTF-16
        // code units on Windows. Neither seem appropriate for this use case,
        // so we do the easy thing for now.
        Data::Text { text: path.to_string_lossy() }
    }
}

impl<'a> serde::Serialize for Data<'a> {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        let mut state = s.serialize_struct("Data", 1)?;
        match *self {
            Data::Text { ref text } => state.serialize_field("text", text)?,
            Data::Bytes { bytes } => {
                // use base64::engine::{general_purpose::STANDARD, Engine};
                // let encoded = STANDARD.encode(bytes);
                state.serialize_field("bytes", &base64_standard(bytes))?;
            }
        }
        state.end()
    }
}

/// Implements "standard" base64 encoding as described in RFC 3548[1].
///
/// We roll our own here instead of bringing in something heavier weight like
/// the `base64` crate. In particular, we really don't care about perf much
/// here, since this is only used for data or file paths that are not valid
/// UTF-8.
///
/// [1]: https://tools.ietf.org/html/rfc3548#section-3
fn base64_standard(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::new();
    let mut it = bytes.chunks_exact(3);
    while let Some(chunk) = it.next() {
        let group24 = (usize::from(chunk[0]) << 16)
            | (usize::from(chunk[1]) << 8)
            | usize::from(chunk[2]);
        let index1 = (group24 >> 18) & 0b111_111;
        let index2 = (group24 >> 12) & 0b111_111;
        let index3 = (group24 >> 6) & 0b111_111;
        let index4 = (group24 >> 0) & 0b111_111;
        out.push(char::from(ALPHABET[index1]));
        out.push(char::from(ALPHABET[index2]));
        out.push(char::from(ALPHABET[index3]));
        out.push(char::from(ALPHABET[index4]));
    }
    match it.remainder() {
        &[] => {}
        &[byte0] => {
            let group8 = usize::from(byte0);
            let index1 = (group8 >> 2) & 0b111_111;
            let index2 = (group8 << 4) & 0b111_111;
            out.push(char::from(ALPHABET[index1]));
            out.push(char::from(ALPHABET[index2]));
            out.push('=');
            out.push('=');
        }
        &[byte0, byte1] => {
            let group16 = (usize::from(byte0) << 8) | usize::from(byte1);
            let index1 = (group16 >> 10) & 0b111_111;
            let index2 = (group16 >> 4) & 0b111_111;
            let index3 = (group16 << 2) & 0b111_111;
            out.push(char::from(ALPHABET[index1]));
            out.push(char::from(ALPHABET[index2]));
            out.push(char::from(ALPHABET[index3]));
            out.push('=');
        }
        _ => unreachable!("remainder must have length < 3"),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests taken from RFC 4648[1].
    //
    // [1]: https://datatracker.ietf.org/doc/html/rfc4648#section-10
    #[test]
    fn base64_basic() {
        let b64 = |s: &str| base64_standard(s.as_bytes());
        assert_eq!(b64(""), "");
        assert_eq!(b64("f"), "Zg==");
        assert_eq!(b64("fo"), "Zm8=");
        assert_eq!(b64("foo"), "Zm9v");
        assert_eq!(b64("foob"), "Zm9vYg==");
        assert_eq!(b64("fooba"), "Zm9vYmE=");
        assert_eq!(b64("foobar"), "Zm9vYmFy");
    }
}
