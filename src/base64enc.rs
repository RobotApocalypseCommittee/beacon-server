use serde::{Serializer, de, Deserialize, Deserializer};

pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
{
    serializer.serialize_str(&base64::encode(bytes))

    // Could also use a wrapper type with a Display implementation to avoid
    // allocating the String.
    //
    // serializer.collect_str(&Base64(bytes))
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de>
{
    let s = <&str>::deserialize(deserializer)?;
    base64::decode(s).map_err(de::Error::custom)
}