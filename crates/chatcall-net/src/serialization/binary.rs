use serde::{Serialize, de::DeserializeOwned};

/// Error type for serialization operations
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    #[error("Serialization failed: {0}")]
    SerializeFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializeFailed(String),
}

/// Serialize a value to compact binary format using bincode
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, SerializationError> {
    bincode::serialize(value)
        .map_err(|e| SerializationError::SerializeFailed(e.to_string()))
}

/// Deserialize from binary format
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, SerializationError> {
    bincode::deserialize(bytes)
        .map_err(|e| SerializationError::DeserializeFailed(e.to_string()))
}

/// Serialize with a size limit to prevent memory exhaustion
pub fn serialize_bounded<T: Serialize>(value: &T, max_size: u64) -> Result<Vec<u8>, SerializationError> {
    bincode::serialized_size(value)
        .map_err(|e| SerializationError::SerializeFailed(e.to_string()))
        .and_then(|size| {
            if size > max_size {
                Err(SerializationError::SerializeFailed(
                    format!("Serialized size {} exceeds limit {}", size, max_size),
                ))
            } else {
                serialize(value)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        id: u32,
        name: String,
        data: Vec<u8>,
    }

    #[test]
    fn test_roundtrip() {
        let original = TestStruct {
            id: 42,
            name: "test".to_string(),
            data: vec![1, 2, 3],
        };
        let bytes = serialize(&original).unwrap();
        let recovered: TestStruct = deserialize(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_compact_encoding() {
        // bincode should be more compact than JSON
        let value = TestStruct {
            id: 1,
            name: "a".to_string(),
            data: vec![0],
        };
        let bytes = serialize(&value).unwrap();
        // bincode for this should be very small (< 30 bytes)
        assert!(bytes.len() < 30);
    }

    #[test]
    fn test_bounded_serialization() {
        let value = TestStruct {
            id: 1,
            name: "test".to_string(),
            data: vec![0; 100],
        };
        // Should fail if limit is too small
        assert!(serialize_bounded(&value, 10).is_err());
        // Should succeed with reasonable limit
        assert!(serialize_bounded(&value, 1000).is_ok());
    }
}
