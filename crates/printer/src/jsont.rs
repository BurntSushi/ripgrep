// This module defines the types we use for JSON serialization. We specifically
// omit deserialization, partially because there isn't a clear use case for
// them at this time, but also because deserialization will complicate things.
// Namely, the types below are designed in a way that permits JSON
// serialization with little or no allocation. Allocation is often quite
// convenient for deserialization however, so these types would become a bit
// more complex.

use std::{borrow::Cow, path::Path};

use {
    base64,
    serde::{Serialize, Serializer},
};

use crate::stats::Stats;

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub(crate) enum Message<'a> {
    Begin(Begin<'a>),
    End(End<'a>),
    Match(Match<'a>),
    Context(Context<'a>),
}

#[derive(Serialize)]
pub(crate) struct Begin<'a> {
    #[serde(serialize_with = "ser_path")]
    pub(crate) path: Option<&'a Path>,
}

#[derive(Serialize)]
pub(crate) struct End<'a> {
    #[serde(serialize_with = "ser_path")]
    pub(crate) path: Option<&'a Path>,
    pub(crate) binary_offset: Option<u64>,
    pub(crate) stats: Stats,
}

#[derive(Serialize)]
pub(crate) struct Match<'a> {
    #[serde(serialize_with = "ser_path")]
    pub(crate) path: Option<&'a Path>,
    #[serde(serialize_with = "ser_bytes")]
    pub(crate) lines: &'a [u8],
    pub(crate) line_number: Option<u64>,
    pub(crate) absolute_offset: u64,
    pub(crate) submatches: &'a [SubMatch<'a>],
}

#[derive(Serialize)]
pub(crate) struct Context<'a> {
    #[serde(serialize_with = "ser_path")]
    pub(crate) path: Option<&'a Path>,
    #[serde(serialize_with = "ser_bytes")]
    pub(crate) lines: &'a [u8],
    pub(crate) line_number: Option<u64>,
    pub(crate) absolute_offset: u64,
    pub(crate) submatches: &'a [SubMatch<'a>],
}

#[derive(Serialize)]
pub(crate) struct SubMatch<'a> {
    #[serde(rename = "match")]
    #[serde(serialize_with = "ser_bytes")]
    pub(crate) m: &'a [u8],
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Data represents things that look like strings, but may actually not be
/// valid UTF-8. To handle this, `Data` is serialized as an object with one
/// of two keys: `text` (for valid UTF-8) or `bytes` (for invalid UTF-8).
///
/// The happy path is valid UTF-8, which streams right through as-is, since
/// it is natively supported by JSON. When invalid UTF-8 is found, then it is
/// represented as arbitrary bytes and base64 encoded.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize)]
#[serde(untagged)]
enum Data<'a> {
    Text {
        text: Cow<'a, str>,
    },
    Bytes {
        #[serde(serialize_with = "to_base64")]
        bytes: &'a [u8],
    },
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

fn to_base64<T, S>(bytes: T, ser: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: Serializer,
{
    use base64::engine::{general_purpose::STANDARD, Engine};
    ser.serialize_str(&STANDARD.encode(&bytes))
}

fn ser_bytes<T, S>(bytes: T, ser: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: Serializer,
{
    Data::from_bytes(bytes.as_ref()).serialize(ser)
}

fn ser_path<P, S>(path: &Option<P>, ser: S) -> Result<S::Ok, S::Error>
where
    P: AsRef<Path>,
    S: Serializer,
{
    path.as_ref().map(|p| Data::from_path(p.as_ref())).serialize(ser)
}
