use crate::{AnalysisSettings, tag_hints::TagHints};
use std::path::Path;
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    proposal::Proposal,
    time::{UtcInstant, UtcOffset},
};

pub struct AnalysisRequest<'a> {
    pub settings: &'a AnalysisSettings,
    pub transcript: &'a Path,
    pub recording: &'a RecordingId,
    pub language: &'a str,
    pub captured: (UtcInstant, UtcOffset),
    pub content: &'a str,
    pub tags: &'a TagHints,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisFailure {
    Disabled,
    InvalidArguments,
    InputLimit,
    Unavailable,
    Io,
    Exited(i32),
    Signaled(i32),
    DeadlineExceeded,
    Interrupted,
    OutputLimit,
    UnsupportedVersion(u64),
    InvalidDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisOutput {
    pub proposal: Proposal,
    pub input_digest: Sha256Digest,
    pub diagnostics: Vec<String>,
}

pub trait AgentCommand {
    fn analyze(&self, request: &AnalysisRequest<'_>) -> Result<AnalysisOutput, AnalysisFailure>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisStateChange {
    pub recording: RecordingId,
    pub expected: super::StageState,
    pub next: super::StageState,
    pub at: vpt_domain::time::UtcInstant,
}
impl AnalysisStateChange {
    pub fn fact(&self) -> (vpt_domain::time::UtcInstant, vpt_domain::run::HistoryFact) {
        (
            self.at,
            vpt_domain::run::HistoryFact::AnalysisStateChanged {
                recording: self.recording.clone(),
                from: self.expected,
                to: self.next,
            },
        )
    }
}
