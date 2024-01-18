use std::{fmt, marker::PhantomData};

use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

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

#[derive(Default, Clone, PartialEq)]
pub struct SecretKey(pub String);

impl Serialize for SecretKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::util::snx_encrypt(self.0.as_bytes()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decrypted = crate::util::snx_decrypt(s.as_bytes()).map_err(serde::de::Error::custom)?;
        Ok(Self(String::from_utf8_lossy(&decrypted).into_owned()))
    }
}

impl From<String> for SecretKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<SecretKey> for String {
    fn from(value: SecretKey) -> Self {
        value.0
    }
}

impl<'a> From<&'a str> for SecretKey {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "****")
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "****")
    }
}

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

impl<'de> Deserialize<'de> for Maybe<u64> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(MaybeVisitor::default())
    }
}

#[derive(Default)]
struct MaybeVisitor<T>(PhantomData<T>);

impl<'de> Visitor<'de> for MaybeVisitor<u64> {
    type Value = Maybe<u64>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "u64 value or empty string")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Maybe(Some(v)))
    }

    fn visit_string<E>(self, _: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Maybe(None))
    }
}
