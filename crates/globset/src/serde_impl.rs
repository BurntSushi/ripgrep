use serde::{
    de::{Error, Visitor},
    {Deserialize, Deserializer, Serialize, Serializer},
};

use crate::Glob;

impl Serialize for Glob {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.glob())
    }
}

struct GlobVisitor;

impl<'a> Visitor<'a> for GlobVisitor {
    type Value = Glob;

    fn expecting(
        &self,
        formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        formatter.write_str("a glob pattern")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Glob::new(v).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for Glob {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        deserializer.deserialize_str(GlobVisitor)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::Glob;

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
    fn glob_deserialize_error() {
        let string = r#"{"error": "["}"#;

        let map = serde_json::from_str::<HashMap<String, Glob>>(&string);

        assert!(map.is_err());
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
