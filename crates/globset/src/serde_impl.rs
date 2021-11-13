use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Glob;

impl Serialize for Glob {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.glob())
    }
}

impl<'de> Deserialize<'de> for Glob {
    fn deserialize<D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        let glob = <&str as Deserialize>::deserialize(deserializer)?;
        Glob::new(glob).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use Glob;

    #[test]
    fn glob_json_works() {
        let test_glob = Glob::new("src/**/*.rs").unwrap();

        let ser = serde_json::to_string(&test_glob).unwrap();
        assert_eq!(ser, "\"src/**/*.rs\"");

        let de: Glob = serde_json::from_str(&ser).unwrap();
        assert_eq!(test_glob, de);
    }
}
