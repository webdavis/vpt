use super::*;
use std::cell::Cell;
use std::path::PathBuf;
use vpt_application::ports::{HistoryWrite, LedgerError};
use vpt_domain::run::{ExportCursor, ExportFailure, ExportPosition, RecoveryBaseline};
use vpt_protocol::encode_history;

mod interrupted;

struct CheckingHistory<'a> {
    inner: &'a MemoryLedger,
    log_path: PathBuf,
    checked: Cell<bool>,
    refuse_begin: bool,
    fail_after_begin: bool,
}

impl RunHistory for CheckingHistory<'_> {
    fn begin_run(&self, kind: RunKind, at: UtcInstant) -> HistoryWrite<HistoryReceipt> {
        if self.refuse_begin && kind == RunKind::Recover {
            return HistoryWrite::Unconfirmed(LedgerError::Busy);
        }
        let written = self.inner.begin_run(kind, at);
        if self.fail_after_begin && kind == RunKind::Recover {
            return post_commit_failure(written);
        }
        written
    }

    fn begin_recovery(
        &self,
        at: UtcInstant,
        evidence: RecoveryEvidence,
    ) -> HistoryWrite<HistoryReceipt> {
        if self.refuse_begin {
            return HistoryWrite::Unconfirmed(LedgerError::Busy);
        }
        let written = self.inner.begin_recovery(at, evidence);
        if self.fail_after_begin {
            return post_commit_failure(written);
        }
        written
    }

    fn commit_step(
        &self,
        run: vpt_domain::history::RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
    ) -> HistoryWrite<HistoryReceipt> {
        if facts
            .iter()
            .any(|(_, fact)| matches!(fact, HistoryFact::RecoverySummary { .. }))
        {
            let before = self.inner.recovery_view().unwrap();
            assert_eq!(before.summarized_through, None);
            let owned: Vec<_> = before
                .records
                .iter()
                .filter(|record| record.run == run)
                .collect();
            assert_eq!(
                owned.len(),
                2,
                "atomic run start and progress precede summary"
            );
            assert!(matches!(
                owned[0].fact,
                HistoryFact::RunStarted(RunKind::Recover)
            ));
            assert!(matches!(
                owned[1].fact,
                HistoryFact::RecoveryProgress { .. }
            ));
            let bytes = fs::read(&self.log_path).unwrap();
            let documents: Vec<serde_json::Value> = bytes
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .map(|line| serde_json::from_slice(line).unwrap())
                .collect();
            for record in owned {
                assert!(
                    documents
                        .iter()
                        .any(|document| document["event_id"] == record.id.render()),
                    "prior committed recovery fact is exported before Summary commit"
                );
            }
            self.checked.set(true);
        }
        self.inner.commit_step(run, delta, facts)
    }

    fn recovery_view(&self) -> Result<RecoveryView, LedgerError> {
        self.inner.recovery_view()
    }

    fn pending_exports(&self) -> Result<Vec<HistoryRecord>, LedgerError> {
        self.inner.pending_exports()
    }

    fn acknowledge_export(
        &self,
        event: vpt_domain::history::EventId,
        expected: Option<ExportCursor>,
        position: ExportPosition,
        appended_at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.inner
            .acknowledge_export(event, expected, position, appended_at)
    }

    fn note_export_failure(&self, failure: &ExportFailure) -> HistoryWrite<()> {
        self.inner.note_export_failure(failure)
    }

    fn restart_export(
        &self,
        expected: ExportCursor,
        observed: ExportPosition,
        observed_at: UtcInstant,
    ) -> HistoryWrite<()> {
        self.inner.restart_export(expected, observed, observed_at)
    }
}

fn post_commit_failure(write: HistoryWrite<HistoryReceipt>) -> HistoryWrite<HistoryReceipt> {
    match write {
        HistoryWrite::Committed(receipt) => HistoryWrite::CommittedWithFailure {
            receipt,
            error: LedgerError::Io("injected after-commit failure".into()),
        },
        other => other,
    }
}

#[test]
fn new_recovery_attempt_exports_start_and_progress_before_summary() {
    let fixture = Fixture::new();
    let (_, _) = saved_log_failure(&fixture);
    let checked = CheckingHistory {
        inner: &fixture.history,
        log_path: fixture.log_path(),
        checked: Cell::new(false),
        refuse_begin: false,
        fail_after_begin: false,
    };
    let before = checked.recovery_view().unwrap();
    let mut sink = LoggedEvents::new(&fixture.state, &checked, &[], &[], || at(20));
    assert_eq!(
        sink.recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    assert!(checked.checked.get());
    let after = checked.recovery_view().unwrap();
    assert_eq!(summaries(&after).len(), 1);
    assert_eq!(
        after
            .records
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::RecoveryProgress { .. }))
            .count(),
        1
    );
}

#[test]
fn unconfirmed_atomic_recovery_begin_preserves_unexported_facts() {
    let fixture = Fixture::new();
    let (_, _) = saved_log_failure(&fixture);
    let original = fixture.history.recovery_view().unwrap();
    let log = fs::read(fixture.log_path()).unwrap();
    let refusing = CheckingHistory {
        inner: &fixture.history,
        log_path: fixture.log_path(),
        checked: Cell::new(false),
        refuse_begin: true,
        fail_after_begin: false,
    };
    let mut sink = LoggedEvents::new(&fixture.state, &refusing, &[], &[], || at(20));
    assert!(matches!(sink.recover(&original), SinkOutcome::Stop(stop)
        if stop.reason == vpt_application::ports::SinkError::RequiredHistory(LedgerError::Busy)
            && stop.committed.is_none()));
    assert_eq!(fixture.history.recovery_view().unwrap(), original);
    assert_eq!(fs::read(fixture.log_path()).unwrap(), log);
}

#[test]
fn committed_recovery_begin_failure_retains_combined_receipt() {
    let fixture = Fixture::new();
    let (_, _) = saved_log_failure(&fixture);
    let before = fixture.history.recovery_view().unwrap();
    let log = fs::read(fixture.log_path()).unwrap();
    let failing = CheckingHistory {
        inner: &fixture.history,
        log_path: fixture.log_path(),
        checked: Cell::new(false),
        refuse_begin: false,
        fail_after_begin: true,
    };
    let mut sink = LoggedEvents::new(&fixture.state, &failing, &[], &[], || at(20));
    let SinkOutcome::Stop(stop) = sink.recover(&before) else {
        panic!("post-commit failure stops recovery");
    };
    assert_eq!(
        stop.reason,
        vpt_application::ports::SinkError::RequiredHistory(LedgerError::Io(
            "injected after-commit failure".into()
        ))
    );
    let receipt = stop.committed.expect("known combined receipt");
    assert_eq!(receipt.records.len(), 2);
    assert_eq!(
        receipt.records[0].fact,
        HistoryFact::RunStarted(RunKind::Recover)
    );
    assert!(matches!(
        receipt.records[1].fact,
        HistoryFact::RecoveryProgress { .. }
    ));
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(
        &after.records[before.records.len()..],
        receipt.records.as_slice()
    );
    assert!(summaries(&after).is_empty());
    assert_eq!(fs::read(fixture.log_path()).unwrap(), log);
}

#[test]
fn start_only_interrupted_recovery_saves_baseline_before_replay() {
    let fixture = Fixture::new();
    let (_, _) = saved_log_failure(&fixture);
    let started = committed(fixture.history.begin_run(RunKind::Recover, at(30)));
    let before = fixture.history.recovery_view().unwrap();
    let pending_from = before.records[0].id;
    let through = started.records[0].id;
    assert!(
        before
            .records
            .iter()
            .all(|record| record.export_position.is_none())
    );

    assert_eq!(
        sink(&fixture, &[], &[]).recover(&before),
        SinkOutcome::Continue {
            warnings: BTreeSet::new()
        }
    );
    let after = fixture.history.recovery_view().unwrap();
    assert_eq!(
        after
            .records
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::RunStarted(RunKind::Recover)))
            .count(),
        1
    );
    let progress: Vec<_> = after
        .records
        .iter()
        .filter_map(|record| match &record.fact {
            HistoryFact::RecoveryProgress { baseline, .. } => Some((record.run, *baseline)),
            _ => None,
        })
        .collect();
    assert_eq!(
        progress,
        [(
            started.run,
            RecoveryBaseline {
                generation: None,
                pending_from: Some(pending_from),
                through: Some(through),
            }
        )]
    );
    assert_eq!(summaries(&after).len(), 1);
    assert_eq!(summaries(&after)[0].run, started.run);
    assert_eq!(
        summary_fact(summaries(&after)[0]).0.recovered_records,
        u32::try_from(before.records.len()).unwrap()
    );
    assert!(fixture.history.pending_exports().unwrap().is_empty());
}
