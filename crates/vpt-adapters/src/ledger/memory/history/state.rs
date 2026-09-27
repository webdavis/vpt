use vpt_application::ports::LedgerError;
use vpt_domain::history::{EventId, ExportFailureId, LedgerNamespace, RunId};
use vpt_domain::run::{
    ExportCursor, ExportRestart, HistoryFact, HistoryRecord, SavedExportFailure,
};
use vpt_domain::time::UtcInstant;

pub(super) const NAMESPACE: LedgerNamespace = LedgerNamespace::new([0x4d; 16]);

#[derive(Clone, Default)]
pub(in crate::ledger::memory) struct HistoryState {
    pub(super) next_run: u64,
    pub(super) next_event: u64,
    pub(super) next_failure: u64,
    pub(super) records: Vec<HistoryRecord>,
    pub(super) failures: Vec<SavedExportFailure>,
    pub(super) restarts: Vec<ExportRestart>,
    pub(super) cursor: Option<ExportCursor>,
}

pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("history state mismatch".into())
}

impl HistoryState {
    pub(super) fn summarized_through(&self) -> Option<ExportFailureId> {
        self.records
            .iter()
            .rev()
            .find_map(|record| match record.fact {
                HistoryFact::RecoverySummary {
                    window: Some(window),
                    ..
                } => Some(window.through),
                _ => None,
            })
    }

    pub(super) fn append(
        &mut self,
        run: RunId,
        at: UtcInstant,
        fact: HistoryFact,
    ) -> Result<HistoryRecord, LedgerError> {
        if run.namespace() != NAMESPACE {
            return Err(conflict());
        }
        if let HistoryFact::EffectObserved { intent, .. } = &fact
            && (intent.namespace() != NAMESPACE
                || !self.records.iter().any(|record| {
                    record.id == *intent && matches!(record.fact, HistoryFact::EffectIntent { .. })
                }))
        {
            return Err(conflict());
        }
        if let HistoryFact::EffectIntent { kind, reference } = &fact
            && !reference.matches_kind(*kind)
        {
            return Err(conflict());
        }
        if let HistoryFact::HookFailed { event, .. } = &fact
            && (event.namespace() != NAMESPACE
                || !self.records.iter().any(|record| {
                    record.id == *event
                        && record.run == run
                        && matches!(record.fact, HistoryFact::Event(_))
                }))
        {
            return Err(conflict());
        }
        if let HistoryFact::RecoverySummary {
            window: Some(window),
            ..
        } = &fact
            && (window.after != self.summarized_through()
                || window.through.namespace() != NAMESPACE
                || window.after.is_some_and(|after| {
                    after.namespace() != NAMESPACE || after.sequence() >= window.through.sequence()
                })
                || !self
                    .failures
                    .iter()
                    .any(|failure| failure.id == window.through))
        {
            return Err(conflict());
        }
        if let HistoryFact::Event(event) = &fact
            && event.occurred_at != at
        {
            return Err(conflict());
        }
        self.next_event = self.next_event.checked_add(1).ok_or_else(conflict)?;
        let record = HistoryRecord {
            id: EventId::new(NAMESPACE, self.next_event),
            run,
            occurred_at: at,
            fact,
            appended_at: None,
            export_position: None,
        };
        self.records.push(record.clone());
        Ok(record)
    }

    pub(super) fn has_run(&self, run: RunId) -> bool {
        self.records
            .iter()
            .any(|record| record.run == run && matches!(record.fact, HistoryFact::RunStarted(_)))
    }
}
