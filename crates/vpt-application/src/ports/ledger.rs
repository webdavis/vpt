use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::{FileTime, UtcInstant, UtcOffset};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    Busy,
    UnsupportedSchema(u32),
    Conflict(String),
    Corrupt(String),
    Io(String),
    PathEscape(std::path::PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeenRow {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
    pub mtime: FileTime,
    pub flags: u32,
    pub first_seen: UtcInstant,
    pub last_seen: UtcInstant,
    pub deferral_count: u32,
    pub deferral_reason: Option<DeferralReason>,
    pub deferred_size: Option<u64>,
    pub source_gone_at: Option<UtcInstant>,
    pub recording: Option<RecordingId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageState {
    Pending,
    Succeeded,
    Failed,
    Disabled,
    Expired,
}

impl StageState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Disabled => "disabled",
            Self::Expired => "expired",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pending" => Some(Self::Pending),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "disabled" => Some(Self::Disabled),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StageStates {
    pub transcribe: StageState,
    pub note: StageState,
    pub synthesis: StageState,
}

impl StageStates {
    pub fn fresh() -> Self {
        Self {
            transcribe: StageState::Pending,
            note: StageState::Pending,
            synthesis: StageState::Pending,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleOrigin {
    VoiceMemos,
    Unavailable,
}

impl TitleOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VoiceMemos => "voice_memos",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "voice_memos" => Some(Self::VoiceMemos),
            "unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingRecord {
    pub id: RecordingId,
    pub source_path: Option<PathBuf>,
    pub digest: Sha256Digest,
    pub captured_at: UtcInstant,
    pub captured_offset: UtcOffset,
    pub duration_secs: u64,
    pub title: Option<String>,
    pub title_source: TitleOrigin,
    pub ingested_at: UtcInstant,
    pub audio_path: PathBuf,
    pub stages: StageStates,
    pub audio_trashed_at: Option<UtcInstant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirtyPublication {
    pub target: PathBuf,
    pub expected_previous: Option<Sha256Digest>,
    pub intended: Sha256Digest,
    pub recorded_at: UtcInstant,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LedgerCommit {
    pub recordings: Vec<RecordingRecord>,
    pub seen: Vec<SeenRow>,
    pub publications: Vec<DirtyPublication>,
}

pub trait RecordingLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError>;
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError>;
    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError>;
    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError>;
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError>;
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError>;
    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError>;
    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError>;
    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError>;
}

pub trait PublicationJournal {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError>;
    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError>;
}

#[cfg(test)]
mod tests {
    use super::{StageState, StageStates, TitleOrigin};

    #[test]
    fn stage_state_uses_exact_persisted_spellings() {
        for (state, spelling) in [
            (StageState::Pending, "pending"),
            (StageState::Succeeded, "succeeded"),
            (StageState::Failed, "failed"),
            (StageState::Disabled, "disabled"),
            (StageState::Expired, "expired"),
        ] {
            assert_eq!(state.as_str(), spelling);
            assert_eq!(StageState::parse(spelling), Some(state));
        }
        assert_eq!(StageState::parse("PENDING"), None);
        assert_eq!(StageState::parse("unknown"), None);
    }

    #[test]
    fn fresh_stages_are_all_pending() {
        assert_eq!(
            StageStates::fresh(),
            StageStates {
                transcribe: StageState::Pending,
                note: StageState::Pending,
                synthesis: StageState::Pending,
            }
        );
    }

    #[test]
    fn title_origin_uses_exact_persisted_spellings() {
        for (origin, spelling) in [
            (TitleOrigin::VoiceMemos, "voice_memos"),
            (TitleOrigin::Unavailable, "unavailable"),
        ] {
            assert_eq!(origin.as_str(), spelling);
            assert_eq!(TitleOrigin::parse(spelling), Some(origin));
        }
        assert_eq!(TitleOrigin::parse("VoiceMemos"), None);
        assert_eq!(TitleOrigin::parse("unknown"), None);
    }
}
