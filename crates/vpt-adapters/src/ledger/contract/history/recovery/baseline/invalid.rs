use super::*;

pub(crate) fn reversed_recovery_baseline_rolls_back_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    reject_baseline(ledger, |start| RecoveryBaseline {
        generation: Some(0),
        pending_from: Some(EventId::new(start.namespace(), 3)),
        through: Some(EventId::new(start.namespace(), 2)),
    });
}

pub(crate) fn missing_pending_recovery_baseline_rolls_back_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    reject_baseline(ledger, |start| RecoveryBaseline {
        generation: None,
        pending_from: None,
        through: Some(start),
    });
}

pub(crate) fn zero_pending_recovery_baseline_rolls_back_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    reject_baseline(ledger, |start| RecoveryBaseline {
        generation: None,
        pending_from: Some(EventId::new(start.namespace(), 0)),
        through: Some(start),
    });
}

pub(crate) fn foreign_pending_recovery_baseline_rolls_back_work<L: RunHistory + RecordingLedger>(
    ledger: &L,
) {
    reject_baseline(ledger, |start| RecoveryBaseline {
        generation: None,
        pending_from: Some(EventId::new(LedgerNamespace::new([0x91; 16]), 1)),
        through: Some(start),
    });
}

fn reject_baseline<L: RunHistory + RecordingLedger>(
    ledger: &L,
    make_baseline: impl FnOnce(EventId) -> RecoveryBaseline,
) {
    let recording = record(95);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![recording.clone()],
            ..Default::default()
        })
        .unwrap();
    let started = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 10 }));
    let before = ledger.recovery_view().unwrap();
    let original = ledger.by_id(&recording.id).unwrap().unwrap().source_path;
    let baseline = make_baseline(started.records[0].id);
    let result = ledger.commit_step(
        started.run,
        &WorkDelta {
            source_updates: vec![(
                recording.id.clone(),
                PathBuf::from("/vm/Recordings/changed-95.m4a"),
            )],
            ..Default::default()
        },
        &[(
            UtcInstant { secs: 11 },
            HistoryFact::RecoveryProgress {
                evidence: evidence(started.run),
                baseline,
            },
        )],
    );
    assert!(matches!(result, HistoryWrite::Unconfirmed(_)), "{result:?}");
    assert_eq!(
        ledger.by_id(&recording.id).unwrap().unwrap().source_path,
        original
    );
    assert_eq!(ledger.recovery_view().unwrap(), before);
}
