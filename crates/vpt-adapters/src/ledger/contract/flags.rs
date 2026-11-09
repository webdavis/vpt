#![allow(
    clippy::single_range_in_vec_init,
    reason = "Flags store ranges, including a single occurrence"
)]
mod resolutions;
use super::{record, transcript::fixture};
pub(crate) use resolutions::*;
use vpt_application::ports::*;
use vpt_domain::{
    review::{FlagContent, Resolution},
    run::{HistoryFact, RunKind, WorkOutcome},
    uncertainty::{Class, Diagnostic, LexicalFlag},
};

pub(crate) fn accepted() -> AcceptedTranscript {
    let mut accepted = fixture();
    accepted.uncertainty.lexical.push(LexicalFlag {
        class: Class::AgreedUnverified,
        ranges: vec![0..1],
        record_text: "Privé".into(),
        alternative_text: None,
        confidence: Some(0.91),
    });
    accepted
        .uncertainty
        .diagnostics
        .push(Diagnostic::SingleEngine);
    accepted
}

pub(crate) fn seed(ledger: &(impl RecordingLedger + RunHistory)) -> vpt_domain::history::RunId {
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(81)],
            ..Default::default()
        })
        .unwrap();
    match ledger.begin_run(RunKind::Transcribe, fixture().accepted_at) {
        HistoryWrite::Committed(receipt) => receipt.run,
        other => panic!("{other:?}"),
    }
}

pub(crate) fn save(
    ledger: &impl RunHistory,
    run: vpt_domain::history::RunId,
    accepted: AcceptedTranscript,
) -> HistoryWrite<HistoryReceipt> {
    let fact = (
        accepted.accepted_at,
        HistoryFact::WorkStep {
            recording: Some(accepted.recording.clone()),
            outcome: WorkOutcome::Completed,
        },
    );
    ledger.commit_step(
        run,
        &WorkDelta {
            transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
            ..Default::default()
        },
        &[fact],
    )
}

pub(crate) fn flags_are_saved_with_accepted_text_and_history(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    let run = seed(ledger);
    assert!(matches!(
        save(ledger, run, accepted()),
        HistoryWrite::Committed(_)
    ));
    let flags = ledger.transcript_flags(&fixture().recording).unwrap();
    assert_eq!(flags.len(), 2);
    assert!(flags.iter().all(|flag| flag.resolution == Resolution::Open));
    assert!(flags.iter().any(
        |flag| flag.content == FlagContent::Lexical(accepted().uncertainty.lexical[0].clone())
    ));
    assert!(
        flags
            .iter()
            .any(|flag| flag.content == FlagContent::Diagnostic(Diagnostic::SingleEngine))
    );
    assert_eq!(
        ledger.accepted_transcript(&fixture().recording).unwrap(),
        Some(accepted())
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}

pub(crate) fn flags_follow_successful_replacement_and_survive_failed_attempt(
    ledger: &(impl RecordingLedger + RunHistory + FlagLedger),
) {
    let run = seed(ledger);
    assert!(matches!(
        save(ledger, run, accepted()),
        HistoryWrite::Committed(_)
    ));
    let before = ledger.transcript_flags(&fixture().recording).unwrap();
    assert_eq!(before.len(), 2);
    assert!(matches!(
        ledger.commit_step(
            run,
            &WorkDelta {
                transcription: Some(TranscriptionChange::Fail(fixture().recording)),
                ..Default::default()
            },
            &[(
                fixture().accepted_at,
                HistoryFact::WorkStep {
                    recording: Some(fixture().recording),
                    outcome: WorkOutcome::Failed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    assert_eq!(
        ledger.transcript_flags(&fixture().recording).unwrap(),
        before
    );
    assert!(matches!(
        save(ledger, run, fixture()),
        HistoryWrite::Committed(_)
    ));
    assert!(
        ledger
            .transcript_flags(&fixture().recording)
            .unwrap()
            .is_empty()
    );
}

pub(crate) fn invalid_flag_ranges_roll_back_accepted_state_and_history(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    let run = seed(ledger);
    let mut invalid = accepted();
    invalid.uncertainty.lexical[0].ranges = vec![0..3];
    assert!(matches!(
        save(ledger, run, invalid),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .accepted_transcript(&fixture().recording)
            .unwrap()
            .is_none()
    );
    assert!(
        ledger
            .transcript_flags(&fixture().recording)
            .unwrap()
            .is_empty()
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

macro_rules! cases {
    ($case:ident) => {
        $case!(resolved_flags_and_rendered_note_commit_or_roll_back_together);
        $case!(confirmation_and_dismissal_retain_explicit_state);
        $case!(corrections_commit_with_history_without_changing_accepted_words);
        $case!(resolution_failures_leave_flags_history_and_accepted_state_unchanged);
        $case!(resolution_history_failures_leave_flags_history_and_accepted_state_unchanged);
        $case!(flags_are_saved_with_accepted_text_and_history);
        $case!(flags_follow_successful_replacement_and_survive_failed_attempt);
        $case!(invalid_flag_ranges_roll_back_accepted_state_and_history);
        $case!(failed_history_append_rolls_back_new_flags);
        $case!(invalid_flag_content_preserves_prior_flags);
    };
}
pub(crate) use cases;

pub(crate) fn failed_history_append_rolls_back_new_flags(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    let run = seed(ledger);
    let value = accepted();
    let facts = [
        (
            value.accepted_at,
            HistoryFact::WorkStep {
                recording: Some(value.recording.clone()),
                outcome: WorkOutcome::Completed,
            },
        ),
        (
            value.accepted_at,
            HistoryFact::HookFailed {
                event: vpt_domain::history::EventId::new(run.namespace(), 999),
                failure: vpt_domain::run::HookFailureClass::Spawn,
            },
        ),
    ];
    assert!(matches!(
        ledger.commit_step(
            run,
            &WorkDelta {
                transcription: Some(TranscriptionChange::Accept(Box::new(value))),
                ..Default::default()
            },
            &facts
        ),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .transcript_flags(&accepted().recording)
            .unwrap()
            .is_empty()
    );
    assert!(
        ledger
            .accepted_transcript(&accepted().recording)
            .unwrap()
            .is_none()
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn invalid_flag_content_preserves_prior_flags(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger + FlagLedger),
) {
    let run = seed(ledger);
    assert!(matches!(
        save(ledger, run, accepted()),
        HistoryWrite::Committed(_)
    ));
    let before = ledger.transcript_flags(&accepted().recording).unwrap();
    for case in 0..6 {
        let mut invalid = accepted();
        match case {
            0 => invalid.uncertainty.lexical[0].record_text = "changed".into(),
            1 => invalid.uncertainty.lexical[0].ranges.push(0..1),
            2 => invalid.lexical_classification = false,
            3 => invalid.uncertainty.lexical[0].alternative_text = Some("changed".into()),
            4 => invalid.uncertainty.lexical[0].confidence = Some(f64::NAN),
            _ => invalid
                .uncertainty
                .diagnostics
                .push(Diagnostic::SingleEngine),
        }
        assert!(
            matches!(save(ledger, run, invalid), HistoryWrite::Unconfirmed(_)),
            "case {case}"
        );
        assert_eq!(
            ledger.transcript_flags(&accepted().recording).unwrap(),
            before
        );
        assert_eq!(
            ledger.accepted_transcript(&accepted().recording).unwrap(),
            Some(accepted())
        );
    }
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}
