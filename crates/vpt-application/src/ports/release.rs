use super::{HistoryReceipt, HistoryWrite, LedgerError, NoteStage};
use std::path::PathBuf;
use vpt_domain::{
    digest::Sha256Digest,
    history::{EventId, RunId},
    identity::RecordingId,
    time::{UtcInstant, UtcOffset},
    uncertainty::Diagnostic,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseProvenance {
    pub captured_at: UtcInstant,
    pub captured_offset: UtcOffset,
    pub engines: Vec<String>,
    pub open_lexical_flags: u32,
    pub open_flags: u32,
    pub diagnostics: Vec<Diagnostic>,
    pub reviewed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReleaseVersion {
    pub content: Sha256Digest,
    pub source: Sha256Digest,
    pub report: Sha256Digest,
}
#[derive(Clone, PartialEq, Eq)]
pub struct NewRelease {
    pub recording: RecordingId,
    pub stage: NoteStage,
    pub destination: PathBuf,
    pub report_root: PathBuf,
    pub source_digest: Sha256Digest,
    pub text: String,
    pub report: String,
    pub provenance: ReleaseProvenance,
    pub created_at: UtcInstant,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseRecord {
    pub sequence: i64,
    pub recording: RecordingId,
    pub stage: NoteStage,
    pub destination: PathBuf,
    pub report_path: PathBuf,
    pub version: ReleaseVersion,
    pub provenance: ReleaseProvenance,
    pub created_at: UtcInstant,
    pub finalized_at: Option<UtcInstant>,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ReleasePayload {
    pub release: ReleaseRecord,
    pub text: String,
    pub report: String,
    pub intents: [EventId; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedRelease {
    pub release: ReleaseRecord,
    pub receipt: HistoryReceipt,
}
impl std::fmt::Debug for NewRelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewRelease")
            .field("recording", &self.recording)
            .field("stage", &self.stage)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for ReleasePayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReleasePayload")
            .field("sequence", &self.release.sequence)
            .finish_non_exhaustive()
    }
}
pub trait ReleaseLedger {
    fn release(&self, sequence: i64) -> Result<Option<ReleaseRecord>, LedgerError>;
    fn release_payload(&self, sequence: i64) -> Result<Option<ReleasePayload>, LedgerError>;
    /// Finalized release sequences, newest first; the caller verifies current file bytes.
    fn releases(&self, recording: &RecordingId) -> Result<Vec<i64>, LedgerError>;
    fn pending_releases(&self) -> Result<Vec<i64>, LedgerError>;
}
pub trait ReleaseHistory {
    fn prepare_release(&self, run: RunId, draft: &NewRelease) -> HistoryWrite<PreparedRelease>;
    /// Called only after both files have been durably published and their digests verified.
    fn complete_release(
        &self,
        run: RunId,
        sequence: i64,
        expected: ReleaseVersion,
        at: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt>;
}
