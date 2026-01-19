use std::collections::BTreeMap;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use super::SExpression;
use super::error::Error;

pub struct Deserializer;

impl Deserializer {
    pub fn deserialize<T: de::DeserializeOwned>(input: &SExpression) -> Result<T, Error> {
        T::deserialize(OwnedDeserializer::new(input.clone()))
    }
}

/// Deserializer that owns the SExpression - needed for DeserializeOwned
pub struct OwnedDeserializer {
    input: SExpression,
}

impl OwnedDeserializer {
    pub fn new(input: SExpression) -> Self {
        Self { input }
    }
}

impl<'de> de::Deserializer<'de> for OwnedDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            SExpression::Null => visitor.visit_unit(),
            SExpression::Value(s) => visit_value(&s, visitor),
            SExpression::Object(name, fields) => visitor.visit_map(OwnedObjectAccess::new(name, fields)),
            SExpression::Array(items) => visitor.visit_seq(OwnedArrayAccess::new(items)),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.input {
            SExpression::Value(s) => match s.as_str() {
                "true" => visitor.visit_bool(true),
                "false" => visitor.visit_bool(false),
                _ => Err(Error::new(format!("Expected bool, got '{}'", s))),
            },
            _ => Err(Error::new("Expected bool value")),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.input {
            SExpression::Value(s) => {
                let n = parse_number(s).ok_or_else(|| Error::new(format!("Expected integer, got '{}'", s)))?;
                visitor.visit_i64(n as i64)
            }
            _ => Err(Error::new("Expected integer value")),
        }
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.input {
            SExpression::Value(s) => {
                let n = parse_number(s).ok_or_else(|| Error::new(format!("Expected integer, got '{}'", s)))?;
                visitor.visit_u64(n)
            }
            _ => Err(Error::new("Expected integer value")),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.input {
            SExpression::Value(s) => {
                let n: f64 = s
                    .parse()
                    .map_err(|_| Error::new(format!("Expected float, got '{}'", s)))?;
                visitor.visit_f64(n)
            }
            _ => Err(Error::new("Expected float value")),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.input {
            SExpression::Value(s) if s.len() == 1 => visitor.visit_char(s.chars().next().unwrap()),
            SExpression::Value(s) => Err(Error::new(format!("Expected char, got '{}'", s))),
            _ => Err(Error::new("Expected char value")),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            SExpression::Value(s) => visitor.visit_string(s),
            _ => Err(Error::new("Expected string value")),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            SExpression::Value(s) => visitor.visit_byte_buf(s.into_bytes()),
            _ => Err(Error::new("Expected bytes value")),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            SExpression::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match &self.input {
            SExpression::Null => visitor.visit_unit(),
            SExpression::Object(_, fields) if fields.is_empty() => visitor.visit_unit(),
            _ => Err(Error::new("Expected unit")),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            SExpression::Array(items) => visitor.visit_seq(OwnedArrayAccess::new(items)),
            _ => Err(Error::new("Expected array")),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            SExpression::Object(name, fields) => visitor.visit_map(OwnedObjectAccess::new(name, fields)),
            _ => Err(Error::new("Expected object")),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.input {
            SExpression::Object(name, obj_fields) => {
                // Check if any field name starts with '(' - indicates named object wrapper
                let named_field = fields.iter().find(|f| f.starts_with('('));

                if let Some(field_name) = named_field {
                    // This is a wrapper struct expecting a named object
                    // The field_name is like "(client_hello", extract the expected name
                    let expected_name = &field_name[1..];

                    // Check if the object name matches
                    if name.as_ref().is_some_and(|n| n == expected_name) {
                        // Create a synthetic object with the named field containing the actual fields
                        visitor.visit_map(OwnedNamedObjectAccess::new(field_name, obj_fields))
                    } else if name.is_none() {
                        // Anonymous object, try to deserialize directly
                        visitor.visit_map(OwnedObjectAccess::new(name, obj_fields))
                    } else {
                        Err(Error::new(format!(
                            "Expected object named '{}', got '{}'",
                            expected_name,
                            name.as_deref().unwrap_or("<anonymous>")
                        )))
                    }
                } else {
                    visitor.visit_map(OwnedObjectAccess::new(name, obj_fields))
                }
            }
            _ => Err(Error::new("Expected object")),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_enum(OwnedEnumAccess::new(self.input))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
}

fn parse_number(s: &str) -> Option<u64> {
    // Try decimal first
    if let Ok(n) = s.parse::<u64>() {
        return Some(n);
    }
    // Try hex with 0x prefix
    if let Some(hex) = s.strip_prefix("0x")
        && let Ok(n) = u64::from_str_radix(hex, 16)
    {
        return Some(n);
    }
    None
}

fn visit_value<'de, V: Visitor<'de>>(s: &str, visitor: V) -> Result<V::Value, Error> {
    // Try parsing as integer first (for JSON compatibility)
    if let Ok(n) = s.parse::<u64>() {
        return visitor.visit_u64(n);
    }
    // Try hex
    if let Some(hex) = s.strip_prefix("0x")
        && let Ok(n) = u64::from_str_radix(hex, 16)
    {
        return visitor.visit_u64(n);
    }
    // Try bool
    match s {
        "true" => return visitor.visit_bool(true),
        "false" => return visitor.visit_bool(false),
        _ => {}
    }
    // Fall back to string
    visitor.visit_string(s.to_owned())
}

struct OwnedArrayAccess {
    items: std::vec::IntoIter<SExpression>,
}

impl OwnedArrayAccess {
    fn new(items: Vec<SExpression>) -> Self {
        Self {
            items: items.into_iter(),
        }
    }
}

impl<'de> SeqAccess<'de> for OwnedArrayAccess {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
        match self.items.next() {
            Some(item) => seed.deserialize(OwnedDeserializer::new(item)).map(Some),
            None => Ok(None),
        }
    }
}

struct OwnedObjectAccess {
    #[allow(dead_code)]
    name: Option<String>,
    iter: std::collections::btree_map::IntoIter<String, SExpression>,
    value: Option<SExpression>,
}

impl OwnedObjectAccess {
    fn new(name: Option<String>, fields: BTreeMap<String, SExpression>) -> Self {
        Self {
            name,
            iter: fields.into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for OwnedObjectAccess {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(de::value::StrDeserializer::new(key.as_str()))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> {
        let value = self
            .value
            .take()
            .ok_or_else(|| Error::new("next_value_seed called before next_key_seed"))?;
        seed.deserialize(OwnedDeserializer::new(value))
    }
}

/// Access for named objects - wraps the fields in a synthetic struct
struct OwnedNamedObjectAccess {
    field_name: &'static str,
    fields: Option<BTreeMap<String, SExpression>>,
}

impl OwnedNamedObjectAccess {
    fn new(field_name: &'static str, fields: BTreeMap<String, SExpression>) -> Self {
        Self {
            field_name,
            fields: Some(fields),
        }
    }
}

impl<'de> MapAccess<'de> for OwnedNamedObjectAccess {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {
        if self.fields.is_some() {
            seed.deserialize(de::value::StrDeserializer::new(self.field_name))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> {
        let fields = self.fields.take().ok_or_else(|| Error::new("No fields"))?;
        let synthetic = SExpression::Object(None, fields);
        seed.deserialize(OwnedDeserializer::new(synthetic))
    }
}

struct OwnedEnumAccess {
    input: SExpression,
}

impl OwnedEnumAccess {
    fn new(input: SExpression) -> Self {
        Self { input }
    }
}

impl<'de> de::EnumAccess<'de> for OwnedEnumAccess {
    type Error = Error;
    type Variant = OwnedVariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> {
        match self.input {
            SExpression::Value(s) => {
                let variant = seed.deserialize(de::value::StrDeserializer::new(s.as_str()))?;
                Ok((variant, OwnedVariantAccess::Unit))
            }
            SExpression::Object(name, fields) => {
                if let Some(name) = name {
                    let variant = seed.deserialize(de::value::StrDeserializer::new(name.as_str()))?;
                    Ok((variant, OwnedVariantAccess::Newtype(SExpression::Object(None, fields))))
                } else {
                    Err(Error::new("Expected named object for enum variant"))
                }
            }
            _ => Err(Error::new("Expected value or object for enum")),
        }
    }
}

enum OwnedVariantAccess {
    Unit,
    Newtype(SExpression),
}

impl<'de> de::VariantAccess<'de> for OwnedVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self {
            OwnedVariantAccess::Unit => Ok(()),
            _ => Err(Error::new("Expected unit variant")),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Self::Error> {
        match self {
            OwnedVariantAccess::Newtype(expr) => seed.deserialize(OwnedDeserializer::new(expr)),
            OwnedVariantAccess::Unit => Err(Error::new("Expected newtype variant")),
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            OwnedVariantAccess::Newtype(expr) => {
                if let SExpression::Array(items) = expr {
                    visitor.visit_seq(OwnedArrayAccess::new(items))
                } else {
                    Err(Error::new("Expected tuple variant"))
                }
            }
            _ => Err(Error::new("Expected tuple variant")),
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self {
            OwnedVariantAccess::Newtype(expr) => {
                if let SExpression::Object(name, fields) = expr {
                    visitor.visit_map(OwnedObjectAccess::new(name, fields))
                } else {
                    Err(Error::new("Expected struct variant"))
                }
            }
            _ => Err(Error::new("Expected struct variant")),
        }
    }
}
