use crate::SqliteLedger;
use std::cell::Cell;
use std::path::Path;
use vpt_application::ports::*;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::history::{EventId, RunId};
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, HistoryRecord, RecoveryEvidence,
    RecoveryView, RunKind,
};
use vpt_domain::time::UtcInstant;

#[derive(Clone, Copy)]
pub(super) enum PostCommit {
    None,
    Preparation,
    Completion,
}

pub(super) struct Calls<'a> {
    pub ledger: &'a SqliteLedger,
    pub writes: Cell<usize>,
    pub post_commit: PostCommit,
}

impl<'a> Calls<'a> {
    pub fn new(ledger: &'a SqliteLedger) -> Self {
        Self {
            ledger,
            writes: Cell::new(0),
            post_commit: PostCommit::None,
        }
    }
}

fn after_commit<T>(result: HistoryWrite<T>) -> HistoryWrite<T> {
    match result {
        HistoryWrite::Committed(receipt) => HistoryWrite::CommittedWithFailure {
            receipt,
            error: LedgerError::Io("owned post-commit observation".into()),
        },
        other => other,
    }
}

impl RecordingLedger for Calls<'_> {
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        self.ledger.recordings()
    }
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        self.ledger.by_id(id)
    }
    fn by_digest(&self, _: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        panic!("unexpected digest lookup")
    }
    fn seen(&self, _: &Path) -> Result<Option<SeenRow>, LedgerError> {
        panic!("unexpected source lookup")
    }
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        panic!("unexpected source listing")
    }
    fn record_seen(&self, _: &SeenRow) -> Result<(), LedgerError> {
        panic!("unexpected source write")
    }
    fn commit(&self, _: &LedgerCommit) -> Result<(), LedgerError> {
        panic!("uncoupled work write")
    }
    fn set_source_path(&self, _: &RecordingId, _: &Path) -> Result<(), LedgerError> {
        panic!("unexpected source write")
    }
    fn set_audio_trashed(&self, _: &RecordingId, _: UtcInstant) -> Result<(), LedgerError> {
        panic!("uncoupled audio mark")
    }
}

impl RetentionJournal for Calls<'_> {
    fn record_intent(&self, _: &NewIntent) -> Result<i64, LedgerError> {
        panic!("uncoupled journal write")
    }
    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        self.ledger.pending_intents()
    }
    fn complete_intent(&self, _: i64, _: UtcInstant) -> Result<(), LedgerError> {
        panic!("uncoupled journal completion")
    }
}

impl RetentionHistory for Calls<'_> {
    fn prepare_retention(&self, run: RunId, intent: &NewIntent) -> HistoryWrite<PreparedRetention> {
        self.writes.set(self.writes.get() + 1);
        let result = self.ledger.prepare_retention(run, intent);
        if matches!(self.post_commit, PostCommit::Preparation) {
            after_commit(result)
        } else {
            result
        }
    }
    fn complete_retention(
        &self,
        run: RunId,
        intent: i64,
        effect: EventId,
        at: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt> {
        self.writes.set(self.writes.get() + 1);
        let result = self.ledger.complete_retention(run, intent, effect, at);
        if matches!(self.post_commit, PostCommit::Completion) {
            after_commit(result)
        } else {
            result
        }
    }
}

impl RunHistory for Calls<'_> {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        self.writes.set(self.writes.get() + 1);
        self.ledger.begin_run(kind, at)
    }
    fn begin_recovery(
        &self,
        at: UtcInstant,
        evidence: RecoveryEvidence,
    ) -> HistoryWrite<HistoryReceipt> {
        self.writes.set(self.writes.get() + 1);
        self.ledger.begin_recovery(at, evidence)
    }
    fn commit_step(
        &self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        self.writes.set(self.writes.get() + 1);
        self.ledger.commit_step(run, delta, facts)
    }
    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        self.ledger.recovery_view()
    }
    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        panic!("unexpected export read")
    }
    fn acknowledge_export(
        &self,
        _: EventId,
        _: Option<ExportCursor>,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        panic!("unexpected export ack")
    }
    fn note_export_failure(&self, _: &ExportFailure) -> HistoryWrite<()> {
        panic!("unexpected export failure write")
    }
    fn restart_export(
        &self,
        _: ExportCursor,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        panic!("unexpected export reset")
    }
}

impl RetentionArtifacts for Calls<'_> {
    fn retention_artifacts(&self) -> Result<Vec<RetentionArtifact>, LedgerError> {
        self.ledger.retention_artifacts()
    }
}
