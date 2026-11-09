use super::EncodeError;
pub(super) fn encode(value: &serde_json::Value) -> Result<String, EncodeError> {
    crate::bounded::encode(value).map_err(|error| match error {
        crate::bounded::EncodeError::Invalid => EncodeError::Invalid,
        crate::bounded::EncodeError::CapacityExceeded => EncodeError::CapacityExceeded,
    })
}
