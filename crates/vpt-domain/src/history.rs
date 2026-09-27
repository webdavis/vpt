use crate::identity::RecordingId;
use crate::time::UtcInstant;
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
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventDetail {
    None,
    Review { spans: u32, agreed_unverified: u32 },
    Failure(SafeFailureClass),
    Deferred,
    Retention { completed: u32, failed: u32 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventCounts {
    pub numeric: u32,
    pub proper_noun: u32,
    pub low_confidence: u32,
    pub formatting: u32,
    pub other: u32,
    pub unsupported: u32,
    pub agreed_unverified: u32,
    pub diagnostics: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPathKey {
    Note,
    Audio,
    Brief,
    Archive,
}

impl EventPathKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Audio => "audio",
            Self::Brief => "brief",
            Self::Archive => "archive",
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
    pub detail: EventDetail,
    pub counts: EventCounts,
    pub paths: Vec<EventPath>,
    pub occurred_at: UtcInstant,
}

impl Event {
    pub fn detail_text(&self) -> String {
        match self.detail {
            EventDetail::None => String::new(),
            EventDetail::Review {
                spans,
                agreed_unverified,
            } => {
                format!("{spans} spans to review, {agreed_unverified} agreed spans unverified")
            }
            EventDetail::Failure(class) => class.as_str().to_owned(),
            EventDetail::Deferred => "recording deferred".into(),
            EventDetail::Retention { completed, failed } => {
                format!("{completed} retention items completed, {failed} failed")
            }
        }
    }
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
mod tests {
    use super::*;

    #[test]
    fn detail_is_derived_from_typed_safe_counts() {
        let event = Event {
            kind: EventKind::ReviewNeeded,
            state: EventState::NeedsAttention,
            recording: None,
            detail: EventDetail::Review {
                spans: 6,
                agreed_unverified: 11,
            },
            counts: EventCounts::default(),
            paths: vec![],
            occurred_at: UtcInstant { secs: 0 },
        };
        assert_eq!(
            event.detail_text(),
            "6 spans to review, 11 agreed spans unverified"
        );
    }
}
