use std::{fmt, marker::PhantomData};

use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

/// String encoded with double quotes
#[derive(Default, Clone, PartialEq)]
pub struct QuotedString(pub String);

impl Serialize for QuotedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("\"{}\"", self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QuotedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(String::deserialize(deserializer)?.trim_matches('"').to_owned()))
    }
}

impl From<String> for QuotedString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<QuotedString> for String {
    fn from(value: QuotedString) -> Self {
        value.0
    }
}

impl<'a> From<&'a str> for QuotedString {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

impl fmt::Display for QuotedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for QuotedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// String encoded with double quotes and separated with commas
#[derive(Default, Clone, PartialEq)]
pub struct QuotedStringList(pub Vec<String>);

impl Serialize for QuotedStringList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("\"{}\"", self.0.join(",")).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QuotedStringList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(
            String::deserialize(deserializer)?
                .trim_matches('"')
                .split(',')
                .map(ToOwned::to_owned)
                .collect(),
        ))
    }
}

impl From<Vec<String>> for QuotedStringList {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}

impl From<QuotedStringList> for Vec<String> {
    fn from(value: QuotedStringList) -> Self {
        value.0
    }
}

impl fmt::Debug for QuotedStringList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// Encrypted string. 'Encryption' here is a simple xor operation.
#[derive(Default, Clone, PartialEq)]
pub struct EncryptedString(pub String);

impl Serialize for EncryptedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::util::snx_encrypt(self.0.as_bytes()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EncryptedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decrypted = crate::util::snx_decrypt(s.as_bytes()).map_err(serde::de::Error::custom)?;
        Ok(Self(String::from_utf8_lossy(&decrypted).into_owned()))
    }
}

impl From<String> for EncryptedString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<EncryptedString> for String {
    fn from(value: EncryptedString) -> Self {
        value.0
    }
}

impl<'a> From<&'a str> for EncryptedString {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

impl fmt::Display for EncryptedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "****")
    }
}

impl fmt::Debug for EncryptedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "****")
    }
}

/// Hex-encoded key in reverse byte order
#[derive(Default, Clone, PartialEq)]
pub struct HexKey(pub String);

impl HexKey {
    fn revert(s: &str) -> String {
        let mut enckey = hex::decode(s).unwrap_or_default();
        enckey.reverse();
        hex::encode(enckey)
    }
}

impl Serialize for HexKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Self::revert(&self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HexKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(Self::revert(&String::deserialize(deserializer)?)))
    }
}

impl From<String> for HexKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<HexKey> for String {
    fn from(value: HexKey) -> Self {
        value.0
    }
}

impl fmt::Display for HexKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for HexKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl<'a> From<&'a str> for HexKey {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

/// Wrapper over possibly empty non-string values
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Maybe<T>(pub Option<T>);

impl<T: Serialize> Serialize for Maybe<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            Some(ref v) => v.serialize(serializer),
            None => "".serialize(serializer),
        }
    }
}

impl<'de, T: TryFrom<u64>> Deserialize<'de> for Maybe<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(MaybeVisitor(PhantomData))
    }
}

#[derive(Default)]
struct MaybeVisitor<T>(PhantomData<T>);

impl<'de, T: TryFrom<u64>> Visitor<'de> for MaybeVisitor<T> {
    type Value = Maybe<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "u64 value or empty string")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Maybe(Some(
            v.try_into()
                .map_err(|_| serde::de::Error::custom("Cannot convert from u64"))?,
        )))
    }

    fn visit_str<E>(self, _: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Maybe(None))
    }

    fn visit_string<E>(self, _: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Maybe(None))
    }
}
