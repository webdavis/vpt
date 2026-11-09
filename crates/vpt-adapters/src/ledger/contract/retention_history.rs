use super::record;
use vpt_application::ports::{
    ArtifactKind, HistoryReceipt, HistoryWrite, LedgerCommit, LedgerError, NewIntent,
    PreparedRetention, RecordingLedger, RetentionHistory, RetentionIntent, RetentionJournal,
    RunHistory, WorkDelta,
};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{
    EffectKind, EffectOutcome, EffectReference, HistoryFact, RunKind, WorkOutcome,
};
use vpt_domain::time::UtcInstant;

mod completion;
mod pair_fixture;
mod refusal;
pub(crate) use completion::*;
pub(crate) use pair_fixture::{PendingPair, RetentionPairFixture, pair_through_ports};
pub(crate) use refusal::*;

fn committed<T: std::fmt::Debug>(result: HistoryWrite<T>) -> T {
    match result {
        HistoryWrite::Committed(value) => value,
        other => panic!("expected committed retention history: {other:?}"),
    }
}

pub(in crate::ledger) fn intent() -> NewIntent {
    let recording = record(120);
    NewIntent {
        kind: ArtifactKind::Audio,
        owner: (recording.id).into(),
        path: recording.audio_path,
        expected: recording.digest,
        recorded_at: UtcInstant { secs: 120 },
    }
}

fn begin<L: RunHistory>(ledger: &L) -> HistoryReceipt {
    committed(ledger.begin_run(RunKind::Retain, UtcInstant { secs: 119 }))
}

fn recorded(id: i64, value: NewIntent) -> RetentionIntent {
    RetentionIntent {
        id,
        kind: value.kind,
        owner: value.owner,
        path: value.path,
        expected: value.expected,
        recorded_at: value.recorded_at,
    }
}

fn prepared<L: RecordingLedger + RunHistory + RetentionJournal>(ledger: &L) -> PreparedRetention {
    seeded(ledger, &intent(), true)
}

fn seeded<L: RunHistory + RetentionJournal>(
    ledger: &L,
    new: &NewIntent,
    with_recording: bool,
) -> PreparedRetention {
    let run = begin(ledger).run;
    let intent_id = ledger.record_intent(new).unwrap();
    let receipt = committed(ledger.commit_step(
        run,
        &WorkDelta {
            commit: LedgerCommit {
                recordings: if with_recording {
                    vec![record(120)]
                } else {
                    vec![]
                },
                ..Default::default()
            },
            ..Default::default()
        },
        &[
            (
                UtcInstant { secs: 119 },
                HistoryFact::WorkStep {
                    recording: Some(new.owner.recording().unwrap().clone()),
                    outcome: WorkOutcome::Completed,
                },
            ),
            (
                new.recorded_at,
                HistoryFact::EffectIntent {
                    kind: EffectKind::RetentionTrash,
                    reference: EffectReference::RetentionIntent {
                        id: u64::try_from(intent_id).unwrap(),
                        path: new.path.clone(),
                    },
                },
            ),
        ],
    ));
    PreparedRetention {
        intent_id,
        effect_id: receipt.records[1].id,
        receipt,
    }
}

pub(crate) fn generated_intent_and_effect_are_one_confirmed_receipt<
    L: RetentionHistory + RetentionJournal + RunHistory,
>(
    ledger: &L,
) {
    let run = begin(ledger).run;
    let new = intent();
    let prepared = committed(ledger.prepare_retention(run, &new));
    assert!(prepared.intent_id > 0);
    assert_eq!(prepared.receipt.run, run);
    assert_eq!(prepared.receipt.completed, vec![]);
    assert_eq!(prepared.receipt.records.len(), 1);
    let saved = &prepared.receipt.records[0];
    assert_eq!(saved.id, prepared.effect_id);
    assert_eq!(saved.run, run);
    assert_eq!(saved.occurred_at, new.recorded_at);
    assert_eq!(
        saved.fact,
        HistoryFact::EffectIntent {
            kind: EffectKind::RetentionTrash,
            reference: EffectReference::RetentionIntent {
                id: u64::try_from(prepared.intent_id).unwrap(),
                path: new.path.clone(),
            },
        }
    );
    assert_eq!(
        ledger.pending_intents().unwrap(),
        vec![recorded(prepared.intent_id, new)]
    );
    assert_eq!(ledger.recovery_view().unwrap().records.last(), Some(saved));
}

pub(crate) fn unknown_run_preparation_leaves_no_journal_or_history<
    L: RetentionHistory + RetentionJournal + RunHistory,
>(
    ledger: &L,
) {
    let start = begin(ledger);
    let unknown = RunId::new(start.run.namespace(), 99);
    let result = ledger.prepare_retention(unknown, &intent());
    assert!(
        matches!(result, HistoryWrite::Unconfirmed(LedgerError::Conflict(_))),
        "{result:?}"
    );
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(ledger.recovery_view().unwrap().records, start.records);
}

pub(crate) fn non_utf8_preparation_leaves_no_journal_or_history<
    L: RetentionHistory + RetentionJournal + RunHistory,
>(
    ledger: &L,
) {
    use std::os::unix::ffi::OsStringExt;
    let start = begin(ledger);
    let mut new = intent();
    new.path = std::ffi::OsString::from_vec(b"/audio/\xff.m4a".to_vec()).into();
    let result = ledger.prepare_retention(start.run, &new);
    assert!(
        matches!(result, HistoryWrite::Unconfirmed(LedgerError::Corrupt(_))),
        "{result:?}"
    );
    assert!(ledger.pending_intents().unwrap().is_empty());
    assert_eq!(ledger.recovery_view().unwrap().records, start.records);
}
