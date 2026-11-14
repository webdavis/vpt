use super::*;
use vpt_domain::{occasion::OccasionId, run::RunKind};

fn occasion() -> NewIntent {
    NewIntent {
        kind: ArtifactKind::Brief,
        owner: OccasionId::parse(contract::record(1).id.as_str())
            .unwrap()
            .into(),
        path: "/briefs/owned.md".into(),
        expected: contract::digest(1),
        recorded_at: UtcInstant { secs: 200 },
    }
}

fn round_trip(ledger: &impl RetentionJournal) {
    let new = occasion();
    let id = ledger.record_intent(&new).unwrap();
    let pending = ledger.pending_intents().unwrap();
    assert_eq!(
        pending,
        [RetentionIntent {
            id,
            kind: new.kind,
            owner: new.owner,
            path: new.path,
            expected: new.expected,
            recorded_at: new.recorded_at
        }]
    );
}

fn refuse_mismatch(ledger: &(impl RetentionJournal + RetentionHistory + RunHistory)) {
    let run = engine::committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 100 })).run;
    let before = ledger.pending_exports().unwrap();
    let mut wrong = occasion();
    wrong.owner = contract::record(1).id.into();
    let mut other = occasion();
    other.kind = ArtifactKind::Audio;
    for intent in [wrong, other] {
        assert!(ledger.record_intent(&intent).is_err());
        assert!(matches!(
            ledger.prepare_retention(run, &intent),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(ledger.pending_intents().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap(), before);
    }
    assert_eq!(ledger.record_intent(&occasion()).unwrap(), 1);
}

#[test]
fn memory_journal_preserves_occasion_owners_and_refuses_inconsistent_kinds() {
    round_trip(&MemoryLedger::new());
    refuse_mismatch(&MemoryLedger::new());
}

#[test]
fn sqlite_journal_preserves_occasion_owners_and_refuses_inconsistent_kinds() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    refuse_mismatch(&ledger);
    drop(ledger);
    let ledger = SqliteLedger::open_read_only(root.path()).unwrap().unwrap();
    assert_eq!(ledger.pending_intents().unwrap()[0].owner, occasion().owner);
}

#[test]
fn sqlite_journal_round_trips_a_brief_intent() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    round_trip(&SqliteLedger::open(&root).unwrap());
}
