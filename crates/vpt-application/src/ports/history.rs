use super::{LedgerCommit, LedgerError};
use std::collections::BTreeSet;
use std::path::PathBuf;
use vpt_domain::history::RunId;
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, HistoryRecord, RecoveryEvidence,
    RecoveryView, RunKind,
};
use vpt_domain::time::UtcInstant;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkDelta {
    pub brief: Option<super::BriefChange>,
    pub brief_published: Option<super::BriefPublication>,
    pub occasion: Option<super::OccasionChange>,
    pub synthesis_state: Option<super::SynthesisStateChange>,
    pub verification: Option<super::VerificationChange>,
    pub proposal: Option<super::ProposalChange>,
    pub relations: Option<super::RelationChange>,
    pub tags: Option<super::TagChange>,
    pub tag_rename: Option<super::TagRename>,
    pub vocabulary: Option<super::VocabularyPublication>,
    pub resolution: Option<super::FlagResolutionChange>,
    pub notes: Vec<super::NoteChange>,
    pub engine_outputs: Vec<super::EngineArtifact>,
    pub transcription: Option<super::TranscriptionChange>,
    pub commit: LedgerCommit,
    pub source_updates: Vec<(RecordingId, PathBuf)>,
    pub publication_clears: Vec<PathBuf>,
    pub audio_trash_marks: Vec<(RecordingId, UtcInstant)>,
}

mod completed;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryReceipt {
    pub run: RunId,
    pub records: Vec<HistoryRecord>,
    pub completed: Vec<RecordingId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryWrite<T> {
    Committed(T),
    CommittedWithFailure { receipt: T, error: LedgerError },
    Unconfirmed(LedgerError),
}

pub trait RunHistory {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt>;
    fn begin_recovery(
        &self,
        at: UtcInstant,
        evidence: RecoveryEvidence,
    ) -> HistoryWrite<HistoryReceipt>;
    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt>;
    fn recovery_view(&self) -> Result<RecoveryView, LedgerError>;
    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError>;
    fn acknowledge_export(
        &self,
        event: vpt_domain::history::EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()>;
    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()>;
    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        observed_at: UtcInstant,
    ) -> HistoryWrite<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SinkWarning {
    ExportUnavailable,
    ExportProgressUnconfirmed,
    ExportFailureEvidenceUnconfirmed,
    ExportBookkeepingPostCommitFailure,
    HookFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinkError {
    UnsafeState,
    RequiredHistory(LedgerError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SinkStop {
    pub reason: SinkError,
    pub committed: Option<HistoryReceipt>,
    pub warnings: BTreeSet<SinkWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SinkOutcome {
    Continue { warnings: BTreeSet<SinkWarning> },
    Stop(Box<SinkStop>),
}

pub trait EventSink {
    fn after_commit(&mut self, committed: &HistoryReceipt) -> SinkOutcome;
    fn recover(&mut self, view: &RecoveryView) -> SinkOutcome;
}

#[cfg(test)]
mod tests;
