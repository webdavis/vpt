use crate::history::{Event, EventId, ExportFailureId, RunId};
use crate::identity::RecordingId;
use crate::time::UtcInstant;
use std::path::PathBuf;

mod recovery;
pub use recovery::{FailureWindow, RecoveryBaseline, RecoveryEvidence, RecoveryView};

mod kinds;
pub use kinds::{RunKind, RunOutcome, WorkOutcome};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    Archive,
    Publication,
    AudioTrash,
    RetentionTrash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectReference {
    ArchiveTarget(PathBuf),
    PublicationTarget(PathBuf),
    AudioTrash {
        recording: RecordingId,
        path: PathBuf,
    },
    RetentionIntent {
        id: u64,
        path: PathBuf,
    },
}

impl EffectReference {
    pub fn matches_kind(&self, kind: EffectKind) -> bool {
        matches!(
            (kind, self),
            (EffectKind::Archive, Self::ArchiveTarget(_))
                | (EffectKind::Publication, Self::PublicationTarget(_))
                | (EffectKind::AudioTrash, Self::AudioTrash { .. })
                | (EffectKind::RetentionTrash, Self::RetentionIntent { .. })
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectOutcome {
    Completed,
    Failed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookFailureClass {
    Spawn,
    Io,
    Encoding,
    Exited(i32),
    Signaled(i32),
    DeadlineExceeded,
    Interrupted,
}

mod facts;
pub use facts::HistoryFact;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportPosition {
    pub device: u64,
    pub inode: u64,
    pub end_offset: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportCursor {
    pub generation: u64,
    pub position: ExportPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFailureClass {
    LeafRefused,
    Io,
    Encoding,
    UnsafeState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartialAppend {
    pub original_end: u64,
    pub retained_end: u64,
    pub tail_repair_synced: bool,
    pub bytes_written: u64,
    pub file_synced: bool,
    pub directory_synced: bool,
    pub identity_revalidated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservedLogFile {
    pub device: u64,
    pub inode: u64,
    pub complete_end_offset: u64,
    pub physical_end_offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportFailure {
    pub event: EventId,
    pub observed_at: UtcInstant,
    pub class: ExportFailureClass,
    pub last_confirmed: Option<ExportPosition>,
    pub observed_file: Option<ObservedLogFile>,
    pub partial: Option<PartialAppend>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedExportFailure {
    pub id: ExportFailureId,
    pub failure: ExportFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportRestart {
    pub old: ExportCursor,
    pub new: ExportCursor,
    pub observed_at: UtcInstant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryRecord {
    pub id: EventId,
    pub run: RunId,
    pub occurred_at: UtcInstant,
    pub fact: HistoryFact,
    pub appended_at: Option<UtcInstant>,
    pub export_position: Option<ExportPosition>,
}
