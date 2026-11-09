use crate::engines::EngineConfig;
use std::fmt;
use std::path::Path;
use std::time::Duration;
use vpt_domain::transcript::Transcript;

pub struct EngineRequest<'a> {
    pub engine: &'a EngineConfig,
    pub audio: &'a Path,
    pub out_dir: &'a Path,
    pub language: &'a str,
    pub duration: f64,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineFailure {
    Unavailable,
    UnsupportedLanguage,
    UnsupportedPlatform,
    MissingSpeechAssets,
    HelperVersion(u32),
    InvalidArguments,
    InvalidAudioDuration,
    Io,
    Exited(i32),
    Signaled(i32),
    DeadlineExceeded,
    Interrupted,
    OutputLimit,
    UnsupportedVersion(u64),
    InvalidDocument,
    WorkspaceCleanup(std::path::PathBuf),
}

pub struct EngineOutput {
    pub transcript: Transcript,
    pub raw_document: Vec<u8>,
    pub diagnostics: Vec<String>,
}

impl fmt::Debug for EngineOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EngineOutput")
            .field("document_bytes", &self.raw_document.len())
            .field("diagnostic_count", &self.diagnostics.len())
            .finish()
    }
}

pub trait Engine {
    fn transcribe(&self, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure>;
}
