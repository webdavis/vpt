use crate::ports::{
    HistoryReceipt, LedgerError, PreparedRetention, SinkError, SinkWarning, StoreError, TrashError,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use vpt_domain::identity::RecordingId;
use vpt_domain::layout::StoreKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    pub recording: RecordingId,
    pub store: StoreKey,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeptReason {
    Untracked,
    DigestMismatch,
}

impl KeptReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Untracked => "untracked",
            Self::DigestMismatch => "digest_mismatch",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    pub path: PathBuf,
    pub reason: KeptReason,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReconcileProgress {
    pub moved: Vec<Moved>,
    pub completed: Vec<RecordingId>,
    pub warnings: BTreeSet<SinkWarning>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RetentionReport {
    pub moved: Vec<Moved>,
    pub kept: Vec<Kept>,
    pub completed: Vec<RecordingId>,
    pub warnings: BTreeSet<SinkWarning>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetentionError {
    Disabled,
    TargetModified(PathBuf),
    MissingHistory { intent_id: i64 },
    Ledger(LedgerError),
    RequiredHistory(LedgerError),
    Stores(StoreError),
    Trash(TrashError),
    SinkStopped(SinkError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetentionCommit {
    Preparation(PreparedRetention),
    Step(HistoryReceipt),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconcileFailure {
    pub error: RetentionError,
    pub progress: ReconcileProgress,
    pub committed: Option<RetentionCommit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetentionFailure {
    pub error: RetentionError,
    pub report: RetentionReport,
    pub committed: Option<RetentionCommit>,
}

impl From<ReconcileProgress> for RetentionReport {
    fn from(progress: ReconcileProgress) -> Self {
        Self {
            moved: progress.moved,
            completed: progress.completed,
            warnings: progress.warnings,
            diagnostics: progress.diagnostics,
            kept: vec![],
        }
    }
}

impl RetentionReport {
    pub(super) fn completed(&mut self, id: &RecordingId) {
        if !self.completed.contains(id) {
            self.completed.push(id.clone());
        }
    }

    pub(super) fn failure(
        &self,
        error: RetentionError,
        committed: Option<RetentionCommit>,
    ) -> Box<RetentionFailure> {
        Box::new(RetentionFailure {
            error,
            report: self.clone(),
            committed,
        })
    }
}

pub(super) fn moved_count(count: usize) -> Result<u32, LedgerError> {
    u32::try_from(count)
        .map_err(|_| LedgerError::Corrupt("retention move count out of range".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_count_is_exact_for_zero_and_largest_wire_count() {
        assert_eq!(moved_count(0), Ok(0));
        assert_eq!(moved_count(u32::MAX as usize), Ok(u32::MAX));
    }

    #[test]
    fn move_count_refuses_above_wire_range() {
        assert_eq!(
            moved_count(u32::MAX as usize + 1),
            Err(LedgerError::Corrupt(
                "retention move count out of range".into()
            ))
        );
    }
}
