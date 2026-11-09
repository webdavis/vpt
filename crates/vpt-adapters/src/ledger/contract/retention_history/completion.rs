use super::*;

pub(crate) fn two_pending_intents_preserve_the_first_audio_mark_and_repeat_history<
    L: RecordingLedger + RetentionHistory + RetentionPairFixture,
>(
    ledger: &L,
) {
    two_pending(ledger, 140);
}

pub(crate) fn two_pending_intents_preserve_first_audio_mark_when_clock_moves_backward<
    L: RecordingLedger + RetentionHistory + RetentionPairFixture,
>(
    ledger: &L,
) {
    two_pending(ledger, 120);
}

fn two_pending<L: RecordingLedger + RetentionHistory + RetentionPairFixture>(
    ledger: &L,
    second_at: i64,
) {
    let PendingPair {
        run,
        intents: [first_id, second_id],
        effects: [first_effect, second_effect],
    } = ledger.seed_retention_pair();
    let new = intent();
    let first =
        committed(ledger.complete_retention(run, first_id, first_effect, UtcInstant { secs: 130 }));
    let second = committed(ledger.complete_retention(
        run,
        second_id,
        second_effect,
        UtcInstant { secs: second_at },
    ));
    assert_eq!(
        first.completed,
        vec![new.owner.recording().unwrap().clone()]
    );
    assert_eq!(second.completed, first.completed);
    assert_eq!(
        ledger
            .by_id(new.owner.recording().unwrap())
            .unwrap()
            .unwrap()
            .audio_trashed_at,
        Some(UtcInstant { secs: 130 }),
        "a second intent must preserve the original audio Trash time"
    );
    for (id, effect) in [(first_id, first_effect), (second_id, second_effect)] {
        let repeat =
            committed(ledger.complete_retention(run, id, effect, UtcInstant { secs: 110 }));
        assert_eq!(repeat.completed, first.completed);
        assert!(repeat.records.is_empty());
    }
}

pub(crate) fn completion_couples_audio_mark_journal_and_confirmed_facts<
    L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let prepared = prepared(ledger);
    let at = UtcInstant { secs: 130 };
    let receipt = committed(ledger.complete_retention(
        prepared.receipt.run,
        prepared.intent_id,
        prepared.effect_id,
        at,
    ));
    assert_eq!(receipt.run, prepared.receipt.run);
    assert_eq!(
        receipt.completed,
        vec![intent().owner.recording().unwrap().clone()]
    );
    assert_eq!(
        receipt
            .records
            .iter()
            .map(|entry| &entry.fact)
            .collect::<Vec<_>>(),
        vec![
            &HistoryFact::EffectObserved {
                intent: prepared.effect_id,
                outcome: EffectOutcome::Completed
            },
            &HistoryFact::WorkStep {
                recording: Some(intent().owner.recording().unwrap().clone()),
                outcome: WorkOutcome::Completed
            },
        ]
    );
    assert!(
        receipt
            .records
            .iter()
            .all(|entry| entry.occurred_at == at && entry.run == receipt.run)
    );
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        ledger
            .by_id(intent().owner.recording().unwrap())
            .unwrap()
            .unwrap()
            .audio_trashed_at,
        Some(at)
    );
    let records = ledger.recovery_view().unwrap().records;
    assert_eq!(&records[records.len() - 2..], receipt.records.as_slice());
}

pub(crate) fn repeated_completion_preserves_original_time_and_history<
    L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let prepared = prepared(ledger);
    let first = committed(ledger.complete_retention(
        prepared.receipt.run,
        prepared.intent_id,
        prepared.effect_id,
        UtcInstant { secs: 130 },
    ));
    let repeated = committed(ledger.complete_retention(
        prepared.receipt.run,
        prepared.intent_id,
        prepared.effect_id,
        UtcInstant { secs: 140 },
    ));
    assert_eq!(repeated.completed, first.completed);
    assert!(
        repeated.records.is_empty(),
        "a retry must not mint another completed fact"
    );
    assert_eq!(
        ledger
            .by_id(intent().owner.recording().unwrap())
            .unwrap()
            .unwrap()
            .audio_trashed_at,
        Some(UtcInstant { secs: 130 })
    );
    let records = ledger.recovery_view().unwrap().records;
    assert_eq!(records.len(), 5);
    assert_eq!(&records[3..], first.records.as_slice());
}

pub(crate) fn recovery_run_observes_original_effect_without_minting_an_intent<
    L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let prepared = prepared(ledger);
    let recovered = committed(ledger.begin_run(RunKind::Recover, UtcInstant { secs: 135 }));
    let receipt = committed(ledger.complete_retention(
        recovered.run,
        prepared.intent_id,
        prepared.effect_id,
        UtcInstant { secs: 140 },
    ));
    assert_eq!(receipt.run, recovered.run);
    assert_eq!(
        receipt.completed,
        vec![intent().owner.recording().unwrap().clone()]
    );
    assert_eq!(
        receipt.records[0].fact,
        HistoryFact::EffectObserved {
            intent: prepared.effect_id,
            outcome: EffectOutcome::Completed,
        }
    );
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(
        ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::EffectIntent { .. }))
            .count(),
        1
    );
}
