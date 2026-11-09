use crate::ports::ArtifactOwner;
use crate::ports::{
    HistoryReceipt, LedgerError, PreparedRetention, SinkError, SinkWarning, StoreError, TrashError,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use vpt_domain::layout::StoreKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    pub owner: ArtifactOwner,
    pub store: StoreKey,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeptReason {
    Untracked,
    DigestMismatch,
    PathEscape,
    OwnershipMismatch,
    PublicationPending,
    Expired,
    Missing,
}

impl KeptReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Untracked => "untracked",
            Self::DigestMismatch => "digest_mismatch",
            Self::PathEscape => "path_escape",
            Self::OwnershipMismatch => "ownership_mismatch",
            Self::PublicationPending => "publication_pending",
            Self::Expired => "expired",
            Self::Missing => "missing",
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
    pub completed: Vec<ArtifactOwner>,
    pub warnings: BTreeSet<SinkWarning>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RetentionReport {
    pub moved: Vec<Moved>,
    pub kept: Vec<Kept>,
    pub completed: Vec<ArtifactOwner>,
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
    pub(super) fn store_counts(&self) -> Result<vpt_domain::history::RetentionCounts, LedgerError> {
        let mut counts = vpt_domain::history::RetentionCounts::default();
        for store in StoreKey::all() {
            counts.set(
                store,
                moved_count(self.moved.iter().filter(|m| m.store == store).count())?,
            );
        }
        Ok(counts)
    }

    pub(super) fn completed(&mut self, id: &ArtifactOwner) {
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
mod tests;
