use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedEngine {
    pub name: String,
    pub family: String,
    pub local: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct EngineDocument {
    pub schema: String,
    pub engine: EngineMetadata,
    pub language: String,
    pub local: bool,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct EngineMetadata {
    pub name: String,
    pub tool: String,
    pub version: String,
    pub model: String,
    pub family: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub words: Vec<Word>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
    #[serde(
        default,
        deserialize_with = "confidence",
        skip_serializing_if = "Option::is_none"
    )]
    pub confidence: Option<f64>,
}

pub(crate) fn confidence<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<f64>, D::Error> {
    f64::deserialize(decoder).map(Some)
}
