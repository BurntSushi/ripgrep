use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;

use crate::Glob;

impl Serialize for Glob {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.glob())
    }
}

struct CowStrVisitor;

impl<'a> Visitor<'a> for CowStrVisitor {
    type Value = Cow<'a, str>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Cow::Borrowed(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Cow::Owned(v))
    }

    fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let s = std::str::from_utf8(v)
            .map_err(|_| Error::invalid_value(Unexpected::Bytes(v), &self))?;
        Ok(Cow::Borrowed(s))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match String::from_utf8(v) {
            Ok(s) => Ok(Cow::Owned(s)),
            Err(e) => Err(Error::invalid_value(
                Unexpected::Bytes(&e.into_bytes()),
                &self,
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Glob {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let cow = deserializer.deserialize_str(CowStrVisitor)?;

        Glob::new(&cow).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use crate::glob::Glob;
    use std::collections::HashMap;

    #[test]
    fn glob_deserialize_borrowed() {
        let string = r#"{"markdown": "*.md"}"#;

        let map: HashMap<String, Glob> =
            serde_json::from_str(&string).unwrap();
        assert_eq!(map["markdown"], Glob::new("*.md").unwrap());
    }

    #[test]
    fn glob_deserialize_owned() {
        let string = r#"{"markdown": "*.md"}"#;

        let v: serde_json::Value = serde_json::from_str(&string).unwrap();
        let map: HashMap<String, Glob> = serde_json::from_value(v).unwrap();
        assert_eq!(map["markdown"], Glob::new("*.md").unwrap());
    }

    #[test]
    fn glob_json_works() {
        let test_glob = Glob::new("src/**/*.rs").unwrap();

        let ser = serde_json::to_string(&test_glob).unwrap();
        assert_eq!(ser, "\"src/**/*.rs\"");

        let de: Glob = serde_json::from_str(&ser).unwrap();
        assert_eq!(test_glob, de);
    }
}
