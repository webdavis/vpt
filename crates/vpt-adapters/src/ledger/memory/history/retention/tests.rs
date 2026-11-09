use super::*;
use crate::ledger::contract;
use vpt_application::ports::{LedgerCommit, RecordingLedger, RunHistory};
use vpt_domain::run::RunKind;

fn committed<T: std::fmt::Debug>(result: HistoryWrite<T>) -> T {
    match result {
        HistoryWrite::Committed(value) => value,
        other => panic!("fixture commit: {other:?}"),
    }
}

fn new_intent() -> NewIntent {
    let record = contract::record(127);
    NewIntent {
        kind: vpt_application::ports::ArtifactKind::Audio,
        owner: (record.id).into(),
        path: record.audio_path,
        expected: record.digest,
        recorded_at: UtcInstant { secs: 120 },
    }
}

#[test]
fn preparation_sequence_exhaustion_rolls_back_the_inserted_intent() {
    let ledger = MemoryLedger::new();
    let run = committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 119 })).run;
    let before = ledger.recovery_view().unwrap();
    ledger.state.lock().unwrap().history.next_event = u64::MAX;
    let result = ledger.prepare_retention(run, &new_intent());
    assert!(
        matches!(result, HistoryWrite::Unconfirmed(LedgerError::Conflict(_))),
        "{result:?}"
    );
    assert_eq!(ledger.recovery_view().unwrap(), before);
    let state = ledger.state.lock().unwrap();
    assert!(state.intents.is_empty());
    assert_eq!(state.next_intent_id, 0);
    assert_eq!(state.history.next_event, u64::MAX);
}

#[test]
fn second_completion_fact_exhaustion_rolls_back_every_prior_write() {
    let ledger = MemoryLedger::new();
    ledger
        .commit(&LedgerCommit {
            recordings: vec![contract::record(127)],
            ..Default::default()
        })
        .unwrap();
    let run = committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 119 })).run;
    let prepared = committed(ledger.prepare_retention(run, &new_intent()));
    let before = ledger.recovery_view().unwrap();
    ledger.state.lock().unwrap().history.next_event = u64::MAX - 1;
    let result = ledger.complete_retention(
        run,
        prepared.intent_id,
        prepared.effect_id,
        UtcInstant { secs: 130 },
    );
    assert!(
        matches!(result, HistoryWrite::Unconfirmed(LedgerError::Conflict(_))),
        "{result:?}"
    );
    assert_eq!(ledger.recovery_view().unwrap(), before);
    let state = ledger.state.lock().unwrap();
    assert_eq!(state.history.next_event, u64::MAX - 1);
    assert!(state.intents[0].1.is_none());
    assert!(state.recordings[0].audio_trashed_at.is_none());
}
