use super::*;

pub(crate) fn wrong_effect_refuses_without_audio_journal_or_history_change<
    L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let prepared = prepared(ledger);
    let wrong = EventId::new(prepared.effect_id.namespace(), 1);
    assert_refused(ledger, &prepared, wrong, &intent());
}

pub(crate) fn wrong_intent_path_refuses_without_audio_journal_or_history_change<
    L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let mut new = intent();
    new.path = "/audio/different.m4a".into();
    let prepared = seeded(ledger, &new, true);
    assert_refused(ledger, &prepared, prepared.effect_id, &new);
}

pub(crate) fn unrelated_valid_effect_refuses_without_audio_journal_or_history_change<
    L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let started = begin(ledger);
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(120)],
            ..Default::default()
        })
        .unwrap();
    let new = intent();
    let first = committed(ledger.prepare_retention(started.run, &new));
    let second = committed(ledger.prepare_retention(started.run, &new));
    assert_ne!(first.intent_id, second.intent_id);
    assert_ne!(first.effect_id, second.effect_id);
    let result = ledger.complete_retention(
        started.run,
        first.intent_id,
        second.effect_id,
        UtcInstant { secs: 130 },
    );
    assert!(
        matches!(result, HistoryWrite::Unconfirmed(LedgerError::Conflict(_))),
        "{result:?}"
    );
    assert_eq!(
        ledger.pending_intents().unwrap(),
        vec![
            recorded(first.intent_id, new.clone()),
            recorded(second.intent_id, new)
        ]
    );
    assert_eq!(
        ledger.by_id(&intent().recording).unwrap(),
        Some(record(120))
    );
    let mut expected = started.records;
    expected.extend(first.receipt.records);
    expected.extend(second.receipt.records);
    assert_eq!(ledger.recovery_view().unwrap().records, expected);
}

pub(crate) fn wrong_intent_digest_refuses_without_audio_journal_or_history_change<
    L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let mut new = intent();
    new.expected = super::super::digest(121);
    let prepared = seeded(ledger, &new, true);
    assert_refused(ledger, &prepared, prepared.effect_id, &new);
}

fn assert_refused<L: RecordingLedger + RunHistory + RetentionHistory + RetentionJournal>(
    ledger: &L,
    prepared: &PreparedRetention,
    effect: EventId,
    new: &NewIntent,
) {
    let result = ledger.complete_retention(
        prepared.receipt.run,
        prepared.intent_id,
        effect,
        UtcInstant { secs: 130 },
    );
    assert!(
        matches!(result, HistoryWrite::Unconfirmed(LedgerError::Conflict(_))),
        "{result:?}"
    );
    assert_eq!(
        ledger.pending_intents().unwrap(),
        vec![recorded(prepared.intent_id, new.clone())]
    );
    assert_eq!(
        ledger.by_id(&intent().recording).unwrap().unwrap(),
        record(120)
    );
    let records = ledger.recovery_view().unwrap().records;
    assert_eq!(records.len(), 3);
    assert_eq!(records.last(), prepared.receipt.records.last());
}

pub(crate) fn missing_recording_refuses_completion_and_preserves_original_intent<
    L: RunHistory + RetentionHistory + RetentionJournal,
>(
    ledger: &L,
) {
    let new = intent();
    let prepared = seeded(ledger, &new, false);
    let result = ledger.complete_retention(
        prepared.receipt.run,
        prepared.intent_id,
        prepared.effect_id,
        UtcInstant { secs: 130 },
    );
    assert!(
        matches!(result, HistoryWrite::Unconfirmed(LedgerError::Conflict(_))),
        "{result:?}"
    );
    assert_eq!(
        ledger.pending_intents().unwrap(),
        vec![recorded(prepared.intent_id, new)]
    );
    assert_eq!(ledger.recovery_view().unwrap().records.len(), 3);
}
