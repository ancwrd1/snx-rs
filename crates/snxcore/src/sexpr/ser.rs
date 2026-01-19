use std::collections::BTreeMap;

use serde::{Serialize, ser};

use super::SExpression;
use super::error::Error;

pub struct Serializer;

impl Serializer {
    pub fn serialize<T: Serialize>(value: &T) -> Result<SExpression, Error> {
        value.serialize(Self)
    }
}

impl ser::Serializer for Serializer {
    type Ok = SExpression;
    type Error = Error;
    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = SeqSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(String::from_utf8_lossy(v).into_owned()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Null)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Object(None, BTreeMap::new()))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Value(variant.to_string()))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer::new(len))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SeqSerializer::new(Some(len)))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SeqSerializer::new(Some(len)))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SeqSerializer::new(Some(len)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new())
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer::new())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructSerializer::new())
    }
}

pub struct SeqSerializer {
    items: Vec<SExpression>,
}

impl SeqSerializer {
    fn new(len: Option<usize>) -> Self {
        Self {
            items: Vec::with_capacity(len.unwrap_or(0)),
        }
    }
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = SExpression;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.items.push(value.serialize(Serializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Array(self.items))
    }
}

impl ser::SerializeTuple for SeqSerializer {
    type Ok = SExpression;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqSerializer {
    type Ok = SExpression;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for SeqSerializer {
    type Ok = SExpression;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

pub struct MapSerializer {
    fields: BTreeMap<String, SExpression>,
    current_key: Option<String>,
}

impl MapSerializer {
    fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
            current_key: None,
        }
    }
}

impl ser::SerializeMap for MapSerializer {
    type Ok = SExpression;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        let key_expr = key.serialize(Serializer)?;
        self.current_key = Some(
            key_expr
                .as_value()
                .ok_or_else(|| Error::new("Map key must be a string"))?
                .clone(),
        );
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .current_key
            .take()
            .ok_or_else(|| Error::new("serialize_value called before serialize_key"))?;
        let value = value.serialize(Serializer)?;
        self.fields.insert(key, value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Object(None, self.fields))
    }
}

pub struct StructSerializer {
    name: Option<String>,
    fields: BTreeMap<String, SExpression>,
}

impl StructSerializer {
    fn new() -> Self {
        Self {
            name: None,
            fields: BTreeMap::new(),
        }
    }
}

impl ser::SerializeStruct for StructSerializer {
    type Ok = SExpression;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        let serialized = value.serialize(Serializer)?;

        // Check if the key starts with '(' - this indicates a named object wrapper
        // e.g., #[serde(rename = "(client_hello")] means the inner value becomes a named object
        if let Some(name) = key.strip_prefix('(') {
            // This field contains the inner data for a named object
            // Extract the fields from the serialized value and set the object name
            if let SExpression::Object(_, inner_fields) = serialized {
                self.name = Some(name.to_string());
                self.fields = inner_fields;
            } else {
                // If not an object, wrap it as a single field
                self.name = Some(name.to_string());
                self.fields.insert(key.to_string(), serialized);
            }
        } else {
            self.fields.insert(key.to_string(), serialized);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SExpression::Object(self.name, self.fields))
    }
}

impl ser::SerializeStructVariant for StructSerializer {
    type Ok = SExpression;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeStruct::end(self)
    }
}
