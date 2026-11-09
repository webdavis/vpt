mod stages;
use super::record;
pub(crate) use stages::*;
use vpt_application::ports::*;
use vpt_domain::run::{HistoryFact, RunKind, WorkOutcome};
use vpt_domain::time::UtcInstant;
use vpt_domain::transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word};

pub(crate) fn fixture() -> AcceptedTranscript {
    AcceptedTranscript {
        checker_engine: None,
        recording: record(81).id,
        requested_language: "fr-CA".into(),
        duration: 1.75,
        accepted_at: UtcInstant { secs: 50 },
        lexical_classification: true,
        from_checker: true,
        uncertainty: Default::default(),
        transcript: Transcript {
            engine: EngineMetadata {
                name: "checker".into(),
                tool: "whisply".into(),
                version: "unreported".into(),
                model: "large-v3".into(),
                family: "whisper".into(),
            },
            language: "fr-CA".into(),
            language_source: LanguageSource::Requested,
            local: true,
            segments: vec![Segment {
                start: 0.25,
                end: 1.75,
                text: "Privé 🦆".into(),
                words: vec![
                    Word {
                        text: "Privé".into(),
                        start: 0.25,
                        end: 1.0,
                        confidence: Some(0.91),
                    },
                    Word {
                        text: "🦆".into(),
                        start: 1.0,
                        end: 1.75,
                        confidence: None,
                    },
                ],
            }],
        },
    }
}

fn committed<T: std::fmt::Debug>(result: HistoryWrite<T>) -> T {
    match result {
        HistoryWrite::Committed(value) => value,
        other => panic!("{other:?}"),
    }
}

fn seed(ledger: &(impl RecordingLedger + RunHistory)) -> vpt_domain::history::RunId {
    ledger
        .commit(&LedgerCommit {
            recordings: vec![record(81)],
            ..Default::default()
        })
        .unwrap();
    committed(ledger.begin_run(RunKind::Transcribe, UtcInstant { secs: 49 })).run
}

fn delta() -> WorkDelta {
    WorkDelta {
        transcription: Some(TranscriptionChange::Accept(Box::new(fixture()))),
        ..Default::default()
    }
}

fn fact(outcome: WorkOutcome) -> (UtcInstant, HistoryFact) {
    (
        UtcInstant { secs: 50 },
        HistoryFact::WorkStep {
            recording: Some(fixture().recording),
            outcome,
        },
    )
}

pub(crate) fn accepted_transcript_and_history_commit_together(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = seed(ledger);
    let receipt = committed(ledger.commit_step(run, &delta(), &[fact(WorkOutcome::Completed)]));
    assert_eq!(
        ledger.accepted_transcript(&fixture().recording).unwrap(),
        Some(fixture())
    );
    assert_eq!(
        ledger
            .by_id(&fixture().recording)
            .unwrap()
            .unwrap()
            .stages
            .transcribe,
        StageState::Succeeded
    );
    assert_eq!(receipt.completed, vec![fixture().recording]);
    let exports = ledger.pending_exports().unwrap();
    assert_eq!(exports.len(), 2);
    assert_eq!(exports[1].fact, fact(WorkOutcome::Completed).1);
    assert!(!format!("{receipt:?}").contains("Privé"));
}

pub(crate) fn failed_attempt_preserves_accepted_words_and_metadata(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &[fact(WorkOutcome::Completed)]));
    let failed = WorkDelta {
        transcription: Some(TranscriptionChange::Fail(fixture().recording)),
        ..Default::default()
    };
    let receipt = committed(ledger.commit_step(run, &failed, &[fact(WorkOutcome::Failed)]));
    assert!(receipt.completed.is_empty());
    assert_eq!(
        ledger.accepted_transcript(&fixture().recording).unwrap(),
        Some(fixture())
    );
    assert_eq!(
        ledger
            .by_id(&fixture().recording)
            .unwrap()
            .unwrap()
            .stages
            .transcribe,
        StageState::Failed
    );
}

pub(crate) fn missing_history_rolls_back_transcript_and_stage(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = seed(ledger);
    assert!(matches!(
        ledger.commit_step(run, &delta(), &[]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .accepted_transcript(&fixture().recording)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        ledger
            .by_id(&fixture().recording)
            .unwrap()
            .unwrap()
            .stages
            .transcribe,
        StageState::Pending
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn invalid_transcript_rolls_back_state_and_history(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &[fact(WorkOutcome::Completed)]));
    let mut changed = fixture();
    changed.transcript.segments[0].words[0].end = 100.0;
    let delta = WorkDelta {
        transcription: Some(TranscriptionChange::Accept(Box::new(changed))),
        ..Default::default()
    };
    assert!(matches!(
        ledger.commit_step(run, &delta, &[fact(WorkOutcome::Completed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger.accepted_transcript(&fixture().recording).unwrap(),
        Some(fixture())
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}

pub(crate) fn a_later_success_replaces_only_the_accepted_transcription(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &[fact(WorkOutcome::Completed)]));
    let mut changed = fixture();
    changed.accepted_at = UtcInstant { secs: 60 };
    changed.from_checker = false;
    changed.transcript.engine.name = "main".into();
    changed.transcript.language_source = LanguageSource::Reported;
    changed.transcript.segments[0].words[0].text = "Corrigé".into();
    let delta = WorkDelta {
        transcription: Some(TranscriptionChange::Accept(Box::new(changed.clone()))),
        ..Default::default()
    };
    committed(ledger.commit_step(run, &delta, &[fact(WorkOutcome::Completed)]));
    assert_eq!(
        ledger.accepted_transcript(&fixture().recording).unwrap(),
        Some(changed)
    );
    let mut expected = record(81);
    expected.stages.transcribe = StageState::Succeeded;
    assert_eq!(ledger.by_id(&expected.id).unwrap(), Some(expected));
}

pub(crate) fn failed_history_append_rolls_back_accepted_transcription(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = seed(ledger);
    let missing = vpt_domain::history::EventId::new(run.namespace(), 900);
    let invalid = HistoryFact::HookFailed {
        event: missing,
        failure: vpt_domain::run::HookFailureClass::Spawn,
    };
    assert!(matches!(
        ledger.commit_step(run, &delta(), &[(UtcInstant { secs: 50 }, invalid)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .accepted_transcript(&fixture().recording)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        ledger
            .by_id(&fixture().recording)
            .unwrap()
            .unwrap()
            .stages
            .transcribe,
        StageState::Pending
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn missing_recording_refuses_transcription_without_history(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = committed(ledger.begin_run(RunKind::Transcribe, UtcInstant { secs: 49 })).run;
    assert!(matches!(
        ledger.commit_step(run, &delta(), &[fact(WorkOutcome::Completed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert!(
        ledger
            .accepted_transcript(&fixture().recording)
            .unwrap()
            .is_none()
    );
    let failed = WorkDelta {
        transcription: Some(TranscriptionChange::Fail(fixture().recording)),
        ..Default::default()
    };
    assert!(matches!(
        ledger.commit_step(run, &failed, &[fact(WorkOutcome::Failed)]),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(ledger.pending_exports().unwrap().len(), 1);
}

pub(crate) fn transcription_refuses_unrelated_or_contradictory_history(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = seed(ledger);
    let unrelated = (
        UtcInstant { secs: 50 },
        HistoryFact::WorkStep {
            recording: Some(record(82).id),
            outcome: WorkOutcome::Completed,
        },
    );
    for facts in [
        vec![unrelated],
        vec![fact(WorkOutcome::Failed)],
        vec![fact(WorkOutcome::Completed), fact(WorkOutcome::Failed)],
    ] {
        assert!(matches!(
            ledger.commit_step(run, &delta(), &facts),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(
            ledger
                .accepted_transcript(&fixture().recording)
                .unwrap()
                .is_none()
        );
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    }
}

pub(crate) fn accepting_a_new_recording_reports_its_identity_once(
    ledger: &(impl RecordingLedger + RunHistory + TranscriptLedger),
) {
    let run = committed(ledger.begin_run(RunKind::Transcribe, UtcInstant { secs: 49 })).run;
    let mut delta = delta();
    delta.commit.recordings.push(record(81));
    let receipt = committed(ledger.commit_step(run, &delta, &[fact(WorkOutcome::Completed)]));
    assert_eq!(receipt.completed, vec![fixture().recording]);
}

macro_rules! cases {
    ($case:ident) => {
        $case!(accepted_transcript_and_history_commit_together);
        $case!(acceptance_invalidates_note_success);
        $case!(acceptance_preserves_expired_notes);
        $case!(rejected_acceptance_preserves_note_success_and_prior_state);
        $case!(failed_attempt_preserves_accepted_words_and_metadata);
        $case!(missing_history_rolls_back_transcript_and_stage);
        $case!(invalid_transcript_rolls_back_state_and_history);
        $case!(a_later_success_replaces_only_the_accepted_transcription);
        $case!(failed_history_append_rolls_back_accepted_transcription);
        $case!(missing_recording_refuses_transcription_without_history);
        $case!(transcription_refuses_unrelated_or_contradictory_history);
        $case!(accepting_a_new_recording_reports_its_identity_once);
    };
}
pub(crate) use cases;
