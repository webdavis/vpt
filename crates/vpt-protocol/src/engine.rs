mod diagnostics;
pub(crate) mod model;
pub(crate) mod validate;
use crate::LimitViolation;
pub use model::{EngineDocument, EngineMetadata, ExpectedEngine, Segment, Word};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineDecodeError {
    Limit(LimitViolation),
    UnsupportedVersion(u64),
    Invalid(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedEngine {
    pub document: EngineDocument,
    pub diagnostics: Vec<String>,
}

pub fn decode(
    bytes: &[u8],
    expected: &ExpectedEngine,
    duration: f64,
) -> Result<DecodedEngine, EngineDecodeError> {
    let value = crate::limits::read_engine(bytes).map_err(EngineDecodeError::Limit)?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or(EngineDecodeError::Invalid("schema"))?;
    let version = schema
        .strip_prefix("vpt.engine/")
        .and_then(|version| version.parse::<u64>().ok())
        .ok_or(EngineDecodeError::Invalid("schema"))?;
    if version != 1 {
        return Err(EngineDecodeError::UnsupportedVersion(version));
    }
    if schema != "vpt.engine/1" {
        return Err(EngineDecodeError::Invalid("schema"));
    }
    let diagnostics = diagnostics::unknown_fields(&value);
    let document =
        serde_json::from_value(value).map_err(|_| EngineDecodeError::Invalid("document"))?;
    validate::validate(&document, expected, duration)?;
    Ok(DecodedEngine {
        document,
        diagnostics,
    })
}

#[cfg(test)]
mod tests;
