use super::super::RuntimeLedger;
use std::path::Path;
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::{
    ArtifactKind, LedgerError, NewIntent, RetentionIntent, RetentionJournal,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::{UtcInstant, UtcOffset};

fn intent(root: &Path, seed: u8) -> NewIntent {
    let expected = Sha256Digest([seed; 32]);
    let recorded_at = UtcInstant {
        secs: 1_787_604_456 + i64::from(seed),
    };
    NewIntent {
        kind: ArtifactKind::Audio,
        owner: (RecordingId::derive(recorded_at, UtcOffset { secs: 0 }, &expected).unwrap()).into(),
        path: root.join(format!("{seed}.m4a")),
        expected,
        recorded_at,
    }
}

fn saved(id: i64, new: NewIntent) -> RetentionIntent {
    RetentionIntent {
        id,
        kind: new.kind,
        owner: new.owner,
        path: new.path,
        expected: new.expected,
        recorded_at: new.recorded_at,
    }
}

#[test]
fn empty_runtime_retention_is_observational_and_refuses_both_writes() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("absent");
    let runtime = RuntimeLedger::read_only(&state).unwrap();
    let new = intent(temp.path(), 1);
    let results = (
        runtime.pending_intents(),
        runtime.record_intent(&new),
        runtime.complete_intent(1, UtcInstant { secs: 99 }),
    );
    assert_eq!(
        results,
        (
            Ok(vec![]),
            Err(LedgerError::Io("read-only ledger".into())),
            Err(LedgerError::Io("read-only ledger".into())),
        )
    );
    assert_eq!(runtime.pending_intents(), Ok(vec![]));
    assert!(!state.exists());
}

#[test]
fn read_only_runtime_retention_keeps_fields_order_and_refuses_both_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let first = intent(temp.path(), 2);
    let second = intent(temp.path(), 3);
    let first_id = ledger.record_intent(&first).unwrap();
    let second_id = ledger.record_intent(&second).unwrap();
    drop(ledger);
    let expected = vec![saved(first_id, first), saved(second_id, second)];
    let runtime = RuntimeLedger::read_only(temp.path()).unwrap();
    let observed = runtime.pending_intents();
    let insert = runtime.record_intent(&intent(temp.path(), 4));
    let complete = runtime.complete_intent(first_id, UtcInstant { secs: 100 });
    assert_eq!(observed, Ok(expected.clone()));
    assert_eq!(insert, Err(LedgerError::Io("read-only ledger".into())));
    assert_eq!(complete, Err(LedgerError::Io("read-only ledger".into())));
    assert_eq!(runtime.pending_intents(), Ok(expected));
}

#[test]
fn writable_runtime_retention_records_durable_intent_with_every_field() {
    let temp = tempfile::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let runtime = RuntimeLedger::Sqlite(SqliteLedger::open(&root).unwrap());
    let new = intent(temp.path(), 5);
    let id = runtime.record_intent(&new).unwrap();
    assert!(id > 0);
    drop(runtime);
    let reopened = SqliteLedger::open_read_only(temp.path()).unwrap().unwrap();
    assert_eq!(reopened.pending_intents(), Ok(vec![saved(id, new)]));
}

#[test]
fn writable_runtime_retention_completes_only_the_selected_intent() {
    let temp = tempfile::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let first_id = ledger.record_intent(&intent(temp.path(), 6)).unwrap();
    let second = intent(temp.path(), 7);
    let second_id = ledger.record_intent(&second).unwrap();
    let runtime = RuntimeLedger::Sqlite(ledger);
    runtime
        .complete_intent(first_id, UtcInstant { secs: 200 })
        .unwrap();
    drop(runtime);
    let reopened = SqliteLedger::open_read_only(temp.path()).unwrap().unwrap();
    assert_eq!(
        reopened.pending_intents(),
        Ok(vec![saved(second_id, second)])
    );
}

#[test]
fn runtime_retention_preserves_root_replacement_refusal() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    std::fs::create_dir(&state).unwrap();
    let root = RootDir::open(&state).unwrap();
    let runtime = RuntimeLedger::Sqlite(SqliteLedger::open(&root).unwrap());
    std::fs::rename(&state, temp.path().join("retained-state")).unwrap();
    std::fs::create_dir(&state).unwrap();
    assert!(
        matches!(runtime.pending_intents(), Err(LedgerError::PathEscape(_))),
        "the runtime must retain the typed containment refusal"
    );
    assert_eq!(std::fs::read_dir(&state).unwrap().count(), 0);
}
