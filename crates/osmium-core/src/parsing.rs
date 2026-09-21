//! Bounded JSON parsing that rejects duplicate keys, including nested objects.

use crate::schema::Diagnostic;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

/// Applies to each metadata document, independently of total package limits.
pub const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON value without duplicate object keys")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(value)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(|number| UniqueValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("JSON numbers must be finite"))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value.into())))
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(UniqueValue(value)) = access.next_element()? {
                    values.push(value);
                }
                Ok(UniqueValue(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!("duplicate object key: {key}")));
                    }
                    let UniqueValue(value) = access.next_value()?;
                    values.insert(key, value);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

/// Input is never modified. Serde's recursion limit remains enabled.
pub fn parse_json(bytes: &[u8], file: &str) -> Result<Value, Box<Diagnostic>> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(Box::new(Diagnostic {
            code: "OSM_INPUT_LIMIT".into(),
            severity: "error".into(),
            file: Some(file.into()),
            line: None,
            column: None,
            path: String::new(),
            message: format!("metadata document exceeds {MAX_DOCUMENT_BYTES} bytes"),
            suggestions: Vec::new(),
        }));
    }
    serde_json::from_slice::<UniqueValue>(bytes)
        .map(|value| value.0)
        .map_err(|error| {
            Box::new(Diagnostic {
                code: "OSM_JSON".into(),
                severity: "error".into(),
                file: Some(file.into()),
                line: Some(error.line()),
                column: Some(error.column()),
                path: String::new(),
                message: error.to_string(),
                suggestions: Vec::new(),
            })
        })
}
