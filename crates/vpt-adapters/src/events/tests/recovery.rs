use super::*;
use vpt_application::ports::SinkWarning;
use vpt_domain::run::{
    ExportFailureClass, FailureWindow, HistoryRecord, RecoveryEvidence, RunOutcome,
    SavedExportFailure,
};

mod counts;
mod gaps;
mod progress;
mod resume;
mod summary;

fn saved_log_failure(fixture: &Fixture) -> (HistoryReceipt, SavedExportFailure) {
    let receipt = fixture.saved_event(EventKind::BriefWritten, EventState::Done);
    committed(fixture.history.commit_step(
        receipt.run,
        &WorkDelta::default(),
        &[(at(12), HistoryFact::RunFinished(RunOutcome::Completed))],
    ));
    fs::write(fixture.log_path(), b"").unwrap();
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        sink(fixture, &[], &[]).after_commit(&receipt),
        SinkOutcome::Continue {
            warnings: BTreeSet::from([SinkWarning::ExportUnavailable])
        }
    );
    let view = fixture.history.recovery_view().unwrap();
    assert_eq!(view.failures.len(), 1);
    assert_eq!(
        view.failures[0].failure.class,
        ExportFailureClass::LeafRefused
    );
    assert_eq!(view.failures[0].failure.event, view.records[0].id);
    assert!(
        view.records
            .iter()
            .all(|record| record.export_position.is_none())
    );
    fs::set_permissions(fixture.log_path(), fs::Permissions::from_mode(0o600)).unwrap();
    (receipt, view.failures[0].clone())
}

fn summaries(view: &vpt_domain::run::RecoveryView) -> Vec<&HistoryRecord> {
    view.records
        .iter()
        .filter(|record| matches!(&record.fact, HistoryFact::RecoverySummary { .. }))
        .collect()
}

fn evidence(run: vpt_domain::history::RunId, failure: &SavedExportFailure) -> RecoveryEvidence {
    RecoveryEvidence {
        affected_runs: vec![run],
        first_failure_observed_at: Some(failure.failure.observed_at),
        logging_restored_observed_at: Some(at(20)),
        failures: 1,
        recovered_records: 3,
        completed_work: 0,
        failed_work: 0,
        unknown_work: 0,
        deferred_work: 0,
        completed_effects: 0,
        failed_effects: 0,
        unresolved_effects: 0,
        missing_history: 0,
    }
}

fn summary_fact(record: &HistoryRecord) -> (&RecoveryEvidence, Option<FailureWindow>) {
    let HistoryFact::RecoverySummary { evidence, window } = &record.fact else {
        panic!("saved recovery summary");
    };
    (evidence, *window)
}
