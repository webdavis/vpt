use crate::{SynthesisSettings, tag_hints::TagHints};
use std::path::Path;
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, proposal::Proposal};

pub struct SynthesisRequest<'a> {
    pub settings: &'a SynthesisSettings,
    pub transcript: &'a Path,
    pub recording: &'a RecordingId,
    pub language: &'a str,
    pub content: &'a str,
    pub tags: &'a TagHints,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynthesisFailure {
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
pub struct SynthesisOutput {
    pub proposal: Proposal,
    pub input_digest: Sha256Digest,
    pub diagnostics: Vec<String>,
}

pub trait AgentCommand {
    fn synthesize(
        &self,
        request: &SynthesisRequest<'_>,
    ) -> Result<SynthesisOutput, SynthesisFailure>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesisStateChange {
    pub recording: RecordingId,
    pub expected: super::StageState,
    pub next: super::StageState,
    pub at: vpt_domain::time::UtcInstant,
}
impl SynthesisStateChange {
    pub fn fact(&self) -> (vpt_domain::time::UtcInstant, vpt_domain::run::HistoryFact) {
        (
            self.at,
            vpt_domain::run::HistoryFact::SynthesisStateChanged {
                recording: self.recording.clone(),
                from: self.expected,
                to: self.next,
            },
        )
    }
}
