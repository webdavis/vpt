use super::{ExportCursor, ExportRestart, HistoryRecord, SavedExportFailure};
use crate::history::{EventId, ExportFailureId, LedgerNamespace, RunId};
use crate::time::UtcInstant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryEvidence {
    pub affected_runs: Vec<RunId>,
    pub first_failure_observed_at: Option<UtcInstant>,
    pub logging_restored_observed_at: Option<UtcInstant>,
    pub failures: u32,
    pub recovered_records: u32,
    pub completed_work: u32,
    pub failed_work: u32,
    pub unknown_work: u32,
    pub deferred_work: u32,
    pub completed_effects: u32,
    pub failed_effects: u32,
    pub unresolved_effects: u32,
    pub missing_history: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecoveryBaseline {
    pub generation: Option<u64>,
    pub pending_from: Option<EventId>,
    pub through: Option<EventId>,
}

impl RecoveryBaseline {
    pub fn is_well_formed(self) -> bool {
        match (self.generation, self.pending_from, self.through) {
            (None, None, None) => true,
            (Some(_), None, Some(through)) => through.sequence() > 0,
            (_, Some(from), Some(through)) => {
                from.sequence() > 0
                    && through.sequence() > 0
                    && from.namespace() == through.namespace()
                    && from.sequence() <= through.sequence()
            }
            _ => false,
        }
    }

    pub fn is_valid_for(self, namespace: LedgerNamespace) -> bool {
        self.is_well_formed()
            && self
                .pending_from
                .is_none_or(|id| id.namespace() == namespace)
            && self.through.is_none_or(|id| id.namespace() == namespace)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailureWindow {
    pub after: Option<ExportFailureId>,
    pub through: ExportFailureId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveryView {
    pub records: Vec<HistoryRecord>,
    pub failures: Vec<SavedExportFailure>,
    pub restarts: Vec<ExportRestart>,
    pub active_cursor: Option<ExportCursor>,
    pub summarized_through: Option<ExportFailureId>,
}
