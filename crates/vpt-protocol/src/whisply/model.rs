use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub(super) struct Document {
    pub device: String,
    pub model: String,
    pub transcription: BTreeMap<String, Transcription>,
}

#[derive(Deserialize)]
pub(super) struct Transcription {
    pub text: String,
    pub chunks: Vec<Chunk>,
}

#[derive(Deserialize)]
pub(super) struct Chunk {
    pub timestamp: [f64; 2],
    pub text: String,
    pub words: Vec<Word>,
}

#[derive(Deserialize)]
pub(super) struct Word {
    pub word: String,
    pub start: f64,
    pub end: f64,
    #[serde(default, deserialize_with = "crate::engine::model::confidence")]
    pub score: Option<f64>,
    #[serde(default)]
    pub speaker: Option<String>,
}
