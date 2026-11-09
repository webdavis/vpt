use crate::engines::SelectionError;
use crate::ports::{EngineFailure, EngineOutput};
use std::path::Path;

pub struct TranscriptionInput<'a> {
    pub audio: &'a Path,
    pub out_dir: &'a Path,
    pub language: Option<&'a str>,
    pub duration: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineSlot {
    Main,
    Checker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptFailure {
    pub engine: String,
    pub slot: EngineSlot,
    pub cause: EngineFailure,
}

#[derive(Debug)]
pub struct Transcription {
    pub language: String,
    pub record_slot: EngineSlot,
    pub record: EngineOutput,
    pub checker: Option<EngineOutput>,
    pub lexical_classification: bool,
    pub(super) checker_lexical_classification: bool,
    pub failures: Vec<AttemptFailure>,
}

impl Transcription {
    pub fn single_engine(&self) -> bool {
        self.checker.is_none()
    }

    pub fn comparison(&self) -> Option<&EngineOutput> {
        if self.lexical_classification && self.checker_lexical_classification {
            self.checker.as_ref()
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TranscriptionError {
    Selection(SelectionError),
    InvalidAudioDuration,
    Failed(Vec<AttemptFailure>),
    Interrupted(Vec<AttemptFailure>),
}
