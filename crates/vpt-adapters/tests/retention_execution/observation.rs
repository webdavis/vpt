use super::support::*;
use std::path::Path;
use vpt_adapters::MemoryLedger;
use vpt_application::ports::*;
use vpt_application::{ReconcileProgress, Retention};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::history::{EventId, RunId};
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{
    ExportCursor, ExportFailure, ExportPosition, HistoryFact, HistoryRecord, RecoveryView, RunKind,
};
use vpt_domain::time::UtcInstant;

struct ReadOnly<'a>(&'a MemoryLedger);

impl RecordingLedger for ReadOnly<'_> {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        self.0.seen(path)
    }
    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        self.0.seen_all()
    }
    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        self.0.by_digest(digest)
    }
    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        self.0.by_id(id)
    }
    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        self.0.recordings()
    }
    fn record_seen(&self, _: &SeenRow) -> Result<(), LedgerError> {
        panic!("observation wrote a seen row")
    }
    fn commit(&self, _: &LedgerCommit) -> Result<(), LedgerError> {
        panic!("observation committed work")
    }
    fn set_source_path(&self, _: &RecordingId, _: &Path) -> Result<(), LedgerError> {
        panic!("observation changed a source path")
    }
    fn set_audio_trashed(&self, _: &RecordingId, _: UtcInstant) -> Result<(), LedgerError> {
        panic!("observation changed an audio marker")
    }
}

impl RetentionJournal for ReadOnly<'_> {
    fn record_intent(&self, _: &NewIntent) -> Result<i64, LedgerError> {
        panic!("observation wrote an intent")
    }
    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        panic!("observation reconciled intents")
    }
    fn complete_intent(&self, _: i64, _: UtcInstant) -> Result<(), LedgerError> {
        panic!("observation completed an intent")
    }
}

impl RetentionHistory for ReadOnly<'_> {
    fn prepare_retention(&self, _: RunId, _: &NewIntent) -> HistoryWrite<PreparedRetention> {
        panic!("observation prepared retention")
    }
    fn complete_retention(
        &self,
        _: RunId,
        _: i64,
        _: EventId,
        _: UtcInstant,
    ) -> HistoryWrite<HistoryReceipt> {
        panic!("observation completed retention")
    }
}

impl RunHistory for ReadOnly<'_> {
    fn begin_run(&self, _: RunKind, _: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        panic!("observation began a run")
    }
    fn commit_step(
        &self,
        _: RunId,
        _: &WorkDelta,
        _: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        panic!("observation wrote history")
    }
    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        panic!("observation read recovery history")
    }
    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        panic!("observation read exports")
    }
    fn acknowledge_export(
        &self,
        _: EventId,
        _: Option<ExportCursor>,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        panic!("observation acknowledged an export")
    }
    fn note_export_failure(&self, _: &ExportFailure) -> HistoryWrite<()> {
        panic!("observation saved an export failure")
    }
    fn restart_export(
        &self,
        _: ExportCursor,
        _: ExportPosition,
        _: UtcInstant,
    ) -> HistoryWrite<()> {
        panic!("observation reset an export")
    }
}

#[test]
fn dry_run_never_consults_mutating_journal_or_history_capabilities() {
    let mut world = World::new();
    let record = world.record("observed.m4a", b"owned observed audio");
    world.prepare(&record);
    let report = Retention {
        ledger: &ReadOnly(&world.ledger),
        stores: &world.stores,
        trash: &world.trash,
        clock: &FixedClock,
        sink: &mut world.sink,
        settings: &world.settings,
        paths: &world.paths,
    }
    .run(true, ReconcileProgress::default())
    .unwrap();
    assert_eq!(report.moved.len(), 1);
    assert!(report.completed.is_empty());
    assert!(world.sink.trace.borrow().is_empty());
}
