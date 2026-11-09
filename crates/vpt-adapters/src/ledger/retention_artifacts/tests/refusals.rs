use super::*;
use vpt_domain::run::RunKind;

fn refuse(
    ledger: &(
         impl RetentionArtifacts
         + RecordingLedger
         + RunHistory
         + PublicationJournal
         + OccasionLedger
         + ReleaseHistory
         + RetentionHistory
         + RetentionJournal
     ),
    variant: usize,
) {
    inventory(ledger);
    let at = UtcInstant { secs: 200 };
    let run = engine::committed(ledger.begin_run(RunKind::Retain, at)).run;
    let artifact = ledger
        .retention_artifacts()
        .unwrap()
        .into_iter()
        .find(|a| a.kind == ArtifactKind::EngineOutput)
        .unwrap();
    let mut intent = NewIntent {
        kind: artifact.kind,
        owner: artifact.owner,
        path: artifact.path,
        expected: artifact.digest,
        recorded_at: at,
    };
    match variant {
        0 => intent.expected = contract::digest(99),
        1 => intent.owner = contract::record(1).id.into(),
        2 => intent.path = "/outputs/another.json".into(),
        3 => ledger
            .commit(&LedgerCommit {
                publications: vec![DirtyPublication {
                    target: intent.path.clone(),
                    expected_previous: Some(intent.expected),
                    intended: intent.expected,
                    recorded_at: at,
                }],
                ..Default::default()
            })
            .unwrap(),
        _ => (),
    }
    let prepared = engine::committed(ledger.prepare_retention(run, &intent));
    let effect = if variant == 4 {
        engine::committed(ledger.begin_run(RunKind::Retain, at)).records[0].id
    } else {
        prepared.effect_id
    };
    let before = (
        ledger.retention_artifacts().unwrap(),
        ledger.pending_exports().unwrap(),
        ledger.pending_intents().unwrap(),
    );
    assert!(
        matches!(
            ledger.complete_retention(run, prepared.intent_id, effect, at),
            HistoryWrite::Unconfirmed(_)
        ),
        "variant {variant}"
    );
    assert_eq!(
        (
            ledger.retention_artifacts().unwrap(),
            ledger.pending_exports().unwrap(),
            ledger.pending_intents().unwrap()
        ),
        before
    );
}

#[test]
fn memory_refuses_mismatched_or_pending_artifacts_without_completion() {
    for variant in 0..5 {
        refuse(&MemoryLedger::new(), variant);
    }
}
#[test]
fn sqlite_refuses_mismatched_or_pending_artifacts_without_completion() {
    for variant in 0..5 {
        let temp = tempfile::tempdir().unwrap();
        let root = RootDir::open(temp.path()).unwrap();
        refuse(&SqliteLedger::open(&root).unwrap(), variant);
    }
}
