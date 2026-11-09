use crate::LimitViolation;
use vpt_domain::context::ContextSnapshot;
mod diagnostics;
mod encode;
mod model;
mod validate;

pub const SCHEMA: &str = "vpt.context/1";

pub fn encode(snapshot: &ContextSnapshot) -> Result<String, DecodeError> {
    let text = encode::encode(snapshot)?;
    decode(text.as_bytes())?;
    Ok(text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Limit(LimitViolation),
    UnsupportedVersion(u64),
    Invalid(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedContext {
    pub snapshot: ContextSnapshot,
    pub diagnostics: Vec<String>,
}

pub fn decode(bytes: &[u8]) -> Result<DecodedContext, DecodeError> {
    let value =
        crate::read_bounded(bytes, &crate::Limits::incoming()).map_err(DecodeError::Limit)?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or(DecodeError::Invalid("schema"))?;
    let version = schema
        .strip_prefix("vpt.context/")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or(DecodeError::Invalid("schema"))?;
    if version != 1 {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    if schema != SCHEMA {
        return Err(DecodeError::Invalid("schema"));
    }
    let diagnostics = diagnostics::unknown_fields(&value);
    let document = serde_json::from_value(value).map_err(|_| DecodeError::Invalid("document"))?;
    Ok(DecodedContext {
        snapshot: validate::convert(document)?,
        diagnostics,
    })
}

#[cfg(test)]
mod tests;
