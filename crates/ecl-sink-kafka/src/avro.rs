//! Avro serialization for Confluent wire format.
//!
//! Converts `serde_json::Map` records to Avro binary with the Confluent
//! wire format header: `[0x00][4-byte schema ID big-endian][avro datum]`.

use apache_avro::types::Value as AvroValue;
use apache_avro::Schema;
use thiserror::Error;

/// Errors during Avro serialization.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AvroError {
    /// Schema parsing failed.
    #[error("invalid Avro schema: {0}")]
    InvalidSchema(String),

    /// Value conversion failed.
    #[error("Avro conversion error: {message}")]
    ConversionError {
        /// Error detail.
        message: String,
    },

    /// Avro encoding failed.
    #[error("Avro encoding error: {0}")]
    EncodingError(Box<apache_avro::Error>),
}

/// Serialize a JSON record to Confluent wire format.
///
/// Wire format: `[0x00][schema_id as 4 bytes BE][avro binary datum]`
///
/// # Errors
///
/// Returns `AvroError` if the record cannot be converted to Avro values
/// or if encoding fails.
pub fn serialize_record_avro(
    record: &serde_json::Map<String, serde_json::Value>,
    schema: &Schema,
    schema_id: i32,
) -> Result<Vec<u8>, AvroError> {
    // Convert JSON record to Avro Value.
    let avro_value = json_to_avro_record(record, schema)?;

    // Encode to Avro binary datum.
    let datum =
        apache_avro::to_avro_datum(schema, avro_value).map_err(|e| AvroError::EncodingError(Box::new(e)))?;

    // Build Confluent wire format: magic byte + schema ID + datum.
    let mut buf = Vec::with_capacity(5 + datum.len());
    buf.push(0x00); // Magic byte
    buf.extend_from_slice(&schema_id.to_be_bytes());
    buf.extend_from_slice(&datum);

    Ok(buf)
}

/// Parse an Avro schema from a JSON string.
///
/// # Errors
///
/// Returns `AvroError::InvalidSchema` if parsing fails.
pub fn parse_schema(schema_json: &str) -> Result<Schema, AvroError> {
    Schema::parse_str(schema_json).map_err(|e| AvroError::InvalidSchema(e.to_string()))
}

/// Convert a JSON record to an Avro record value, guided by the schema.
fn json_to_avro_record(
    record: &serde_json::Map<String, serde_json::Value>,
    schema: &Schema,
) -> Result<AvroValue, AvroError> {
    match schema {
        Schema::Record(record_schema) => {
            let mut avro_fields = Vec::new();

            for field in &record_schema.fields {
                let json_val = record.get(&field.name);
                let avro_val = json_value_to_avro(json_val, &field.schema)?;
                avro_fields.push((field.name.clone(), avro_val));
            }

            Ok(AvroValue::Record(avro_fields))
        }
        _ => Err(AvroError::ConversionError {
            message: "top-level schema must be a record".to_string(),
        }),
    }
}

/// Convert a single JSON value to an Avro value based on the target schema.
fn json_value_to_avro(
    value: Option<&serde_json::Value>,
    schema: &Schema,
) -> Result<AvroValue, AvroError> {
    match schema {
        Schema::Null => Ok(AvroValue::Null),
        Schema::Boolean => match value {
            Some(serde_json::Value::Bool(b)) => Ok(AvroValue::Boolean(*b)),
            _ => Ok(AvroValue::Boolean(false)),
        },
        Schema::Int => match value {
            Some(serde_json::Value::Number(n)) => {
                Ok(AvroValue::Int(n.as_i64().unwrap_or(0) as i32))
            }
            Some(serde_json::Value::String(s)) => {
                Ok(AvroValue::Int(s.parse::<i32>().unwrap_or(0)))
            }
            _ => Ok(AvroValue::Int(0)),
        },
        Schema::Long => match value {
            Some(serde_json::Value::Number(n)) => Ok(AvroValue::Long(n.as_i64().unwrap_or(0))),
            Some(serde_json::Value::String(s)) => {
                Ok(AvroValue::Long(s.parse::<i64>().unwrap_or(0)))
            }
            _ => Ok(AvroValue::Long(0)),
        },
        Schema::Float => match value {
            Some(serde_json::Value::Number(n)) => {
                Ok(AvroValue::Float(n.as_f64().unwrap_or(0.0) as f32))
            }
            _ => Ok(AvroValue::Float(0.0)),
        },
        Schema::Double => match value {
            Some(serde_json::Value::Number(n)) => {
                Ok(AvroValue::Double(n.as_f64().unwrap_or(0.0)))
            }
            _ => Ok(AvroValue::Double(0.0)),
        },
        Schema::String => match value {
            Some(serde_json::Value::String(s)) => Ok(AvroValue::String(s.clone())),
            Some(v) => Ok(AvroValue::String(v.to_string())),
            None => Ok(AvroValue::String(String::new())),
        },
        Schema::Union(union_schema) => {
            // Handle ["null", "type"] union (optional fields).
            let variants = union_schema.variants();
            if value.is_none() || matches!(value, Some(serde_json::Value::Null)) {
                // Find null variant.
                for (i, variant) in variants.iter().enumerate() {
                    if *variant == Schema::Null {
                        return Ok(AvroValue::Union(i as u32, Box::new(AvroValue::Null)));
                    }
                }
                return Err(AvroError::ConversionError {
                    message: "union does not contain null".to_string(),
                });
            }
            // Find the first non-null variant that could match.
            for (i, variant) in variants.iter().enumerate() {
                if *variant != Schema::Null {
                    let converted = json_value_to_avro(value, variant)?;
                    return Ok(AvroValue::Union(i as u32, Box::new(converted)));
                }
            }
            Err(AvroError::ConversionError {
                message: "no matching union variant".to_string(),
            })
        }
        _ => {
            // Fallback: serialize as string.
            match value {
                Some(v) => Ok(AvroValue::String(v.to_string())),
                None => Ok(AvroValue::Null),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn test_schema() -> Schema {
        parse_schema(
            r#"{
                "type": "record",
                "name": "Transaction",
                "fields": [
                    {"name": "id", "type": "string"},
                    {"name": "amount", "type": "double"},
                    {"name": "count", "type": "int"},
                    {"name": "active", "type": "boolean"}
                ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn test_avro_serialize_simple_record() {
        let schema = test_schema();
        let mut record = serde_json::Map::new();
        record.insert("id".to_string(), serde_json::json!("txn-001"));
        record.insert("amount".to_string(), serde_json::json!(42.50));
        record.insert("count".to_string(), serde_json::json!(3));
        record.insert("active".to_string(), serde_json::json!(true));

        let bytes = serialize_record_avro(&record, &schema, 42).unwrap();
        assert!(!bytes.is_empty());
        // Verify wire format header.
        assert_eq!(bytes[0], 0x00); // Magic byte
    }

    #[test]
    fn test_avro_serialize_wire_format_header() {
        let schema = test_schema();
        let mut record = serde_json::Map::new();
        record.insert("id".to_string(), serde_json::json!("x"));
        record.insert("amount".to_string(), serde_json::json!(0.0));
        record.insert("count".to_string(), serde_json::json!(0));
        record.insert("active".to_string(), serde_json::json!(false));

        let bytes = serialize_record_avro(&record, &schema, 256).unwrap();

        // Header: [0x00] [schema_id BE 4 bytes]
        assert_eq!(bytes[0], 0x00);
        let schema_id = i32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        assert_eq!(schema_id, 256);
    }

    #[test]
    fn test_avro_serialize_with_nullable_fields() {
        let schema = parse_schema(
            r#"{
                "type": "record",
                "name": "OptionalRecord",
                "fields": [
                    {"name": "name", "type": "string"},
                    {"name": "note", "type": ["null", "string"], "default": null}
                ]
            }"#,
        )
        .unwrap();

        let mut record = serde_json::Map::new();
        record.insert("name".to_string(), serde_json::json!("Alice"));
        // note is absent → null

        let bytes = serialize_record_avro(&record, &schema, 1).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_avro_serialize_nullable_with_value() {
        let schema = parse_schema(
            r#"{
                "type": "record",
                "name": "OptionalRecord",
                "fields": [
                    {"name": "name", "type": "string"},
                    {"name": "note", "type": ["null", "string"], "default": null}
                ]
            }"#,
        )
        .unwrap();

        let mut record = serde_json::Map::new();
        record.insert("name".to_string(), serde_json::json!("Alice"));
        record.insert("note".to_string(), serde_json::json!("hello"));

        let bytes = serialize_record_avro(&record, &schema, 1).unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_parse_schema_invalid() {
        let result = parse_schema("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_avro_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AvroError>();
    }
}
