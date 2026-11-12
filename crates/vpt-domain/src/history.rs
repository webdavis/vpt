mod detail;
mod done;
mod retention_counts;
use crate::identity::RecordingId;
use crate::time::UtcInstant;
pub use detail::EventDetail;
pub use retention_counts::RetentionCounts;
use std::path::PathBuf;

mod id;
pub use id::{EventId, ExportFailureId, LedgerNamespace, RunId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    ReviewNeeded,
    TranscribeFailed,
    IngestFailed,
    Deferred,
    BriefWritten,
    Retention,
    ConfigRefused,
    Transcribed,
    Synthesized,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReviewNeeded => "review_needed",
            Self::TranscribeFailed => "transcribe_failed",
            Self::IngestFailed => "ingest_failed",
            Self::Deferred => "deferred",
            Self::BriefWritten => "brief_written",
            Self::Retention => "retention",
            Self::ConfigRefused => "config_refused",
            Self::Transcribed => "transcribed",
            Self::Synthesized => "synthesized",
        }
    }

    pub fn is_report(self) -> bool {
        matches!(
            self,
            Self::BriefWritten | Self::Retention | Self::Transcribed | Self::Synthesized
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventState {
    NeedsAttention,
    Failed,
    Done,
}

impl EventState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NeedsAttention => "needs_attention",
            Self::Failed => "failed",
            Self::Done => "done",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafeFailureClass {
    SourceUnavailable,
    ArchiveFailed,
    RecordingFailed,
    LedgerUnavailable,
    ExportUnavailable,
    ConfigurationRefused,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventCounts {
    pub retention: Option<RetentionCounts>,
    pub numeric: u32,
    pub proper_noun: u32,
    pub low_confidence: u32,
    pub formatting: u32,
    pub other: u32,
    pub unsupported: u32,
    pub unsourced: u32,
    pub bad_reference: u32,
    pub built_on_flagged_text: u32,
    pub agreed_unverified: u32,
    pub diagnostics: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPathKey {
    Note,
    Audio,
    Brief,
    Archive,
    Source,
}

impl EventPathKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Audio => "audio",
            Self::Brief => "brief",
            Self::Archive => "archive",
            Self::Source => "source",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventPath {
    pub key: EventPathKey,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub state: EventState,
    pub recording: Option<RecordingId>,
    pub occasion: Option<crate::occasion::OccasionId>,
    pub detail: EventDetail,
    pub counts: EventCounts,
    pub paths: Vec<EventPath>,
    pub occurred_at: UtcInstant,
}

impl SafeFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceUnavailable => "source unavailable",
            Self::ArchiveFailed => "archive failed",
            Self::RecordingFailed => "recording failed",
            Self::LedgerUnavailable => "ledger unavailable",
            Self::ExportUnavailable => "event export unavailable",
            Self::ConfigurationRefused => "configuration refused",
        }
    }
}

#[cfg(test)]
mod tests;
