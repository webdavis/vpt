mod checker;
use super::{record, transcript::fixture};
pub(crate) use checker::checker_metadata_must_match_the_published_output;
use sha2::{Digest, Sha256};
use vpt_application::ports::*;
use vpt_domain::run::{HistoryFact, RunKind, WorkOutcome};

pub(crate) fn artifact() -> EngineArtifact {
    let accepted = fixture();
    let mut bytes = serde_json::to_vec_pretty(&crate::transcript_document::to_document(
        &accepted.transcript,
    ))
    .unwrap();
    bytes.push(b'\n');
    EngineArtifact {
        target: format!("/outputs/{}.checker.json", accepted.recording).into(),
        recording: accepted.recording,
        engine: "checker".into(),
        duration: accepted.duration,
        language_source: accepted.transcript.language_source,
        digest: vpt_domain::digest::Sha256Digest(Sha256::digest(&bytes).into()),
        bytes,
        created_at: accepted.accepted_at,
    }
}

pub(crate) fn delta() -> WorkDelta {
    let output = artifact();
    WorkDelta {
        transcription: Some(TranscriptionChange::Accept(Box::new(fixture()))),
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: output.target.clone(),
                expected_previous: None,
                intended: output.digest,
                recorded_at: output.created_at,
            }],
            ..Default::default()
        },
        engine_outputs: vec![output],
        ..Default::default()
    }
}

pub(crate) fn fact() -> [(vpt_domain::time::UtcInstant, HistoryFact); 1] {
    [(
        fixture().accepted_at,
        HistoryFact::WorkStep {
            recording: Some(fixture().recording),
            outcome: WorkOutcome::Completed,
        },
    )]
}

pub(crate) fn committed<T: std::fmt::Debug>(result: HistoryWrite<T>) -> T {
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
    committed(ledger.begin_run(RunKind::Transcribe, fixture().accepted_at)).run
}

pub(crate) fn raw_bytes_and_publication_commit_with_accepted_state(
    ledger: &(
         impl RecordingLedger + RunHistory + EngineOutputLedger + TranscriptLedger + PublicationJournal
     ),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &fact()));
    assert_eq!(
        ledger.engine_output(&artifact().target).unwrap(),
        Some(artifact())
    );
    assert_eq!(
        ledger.accepted_transcript(&fixture().recording).unwrap(),
        Some(fixture())
    );
    assert_eq!(
        ledger.pending_publications().unwrap(),
        delta().commit.publications
    );
    assert!(!format!("{:?}", ledger.pending_exports().unwrap()).contains("Privé"));
}

pub(crate) fn missing_or_wrong_publication_refuses_raw_output(
    ledger: &(
         impl RecordingLedger + RunHistory + EngineOutputLedger + TranscriptLedger + PublicationJournal
     ),
) {
    let run = seed(ledger);
    for wrong in [false, true] {
        let mut delta = delta();
        if wrong {
            delta.commit.publications[0].intended = super::digest(99);
        } else {
            delta.commit.publications.clear();
        }
        assert!(matches!(
            ledger.commit_step(run, &delta, &fact()),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(ledger.engine_output(&artifact().target).unwrap().is_none());
        assert!(
            ledger
                .accepted_transcript(&fixture().recording)
                .unwrap()
                .is_none()
        );
        assert!(ledger.pending_publications().unwrap().is_empty());
    }
}

pub(crate) fn pending_publication_prevents_replacing_saved_engine_bytes(
    ledger: &(
         impl RecordingLedger + RunHistory + EngineOutputLedger + TranscriptLedger + PublicationJournal
     ),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &fact()));
    assert!(matches!(
        ledger.commit_step(run, &delta(), &fact()),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger.engine_output(&artifact().target).unwrap(),
        Some(artifact())
    );
    assert_eq!(ledger.pending_exports().unwrap().len(), 2);
}

pub(crate) fn replacing_a_published_output_requires_its_saved_previous_digest(
    ledger: &(
         impl RecordingLedger + RunHistory + EngineOutputLedger + TranscriptLedger + PublicationJournal
     ),
) {
    let run = seed(ledger);
    committed(ledger.commit_step(run, &delta(), &fact()));
    ledger.clear_publication(&artifact().target).unwrap();
    let mut changed = delta();
    changed.engine_outputs[0].bytes.push(b' ');
    changed.engine_outputs[0].digest =
        vpt_domain::digest::Sha256Digest(Sha256::digest(&changed.engine_outputs[0].bytes).into());
    changed.commit.publications[0].intended = changed.engine_outputs[0].digest;
    assert!(matches!(
        ledger.commit_step(run, &changed, &fact()),
        HistoryWrite::Unconfirmed(_)
    ));
    assert_eq!(
        ledger.engine_output(&artifact().target).unwrap(),
        Some(artifact())
    );
    changed.commit.publications[0].expected_previous = Some(artifact().digest);
    committed(ledger.commit_step(run, &changed, &fact()));
    assert_eq!(
        ledger.engine_output(&artifact().target).unwrap(),
        Some(changed.engine_outputs.remove(0))
    );
}

pub(crate) fn mismatched_bytes_identity_or_record_text_roll_back_the_batch(
    ledger: &(
         impl RecordingLedger + RunHistory + EngineOutputLedger + TranscriptLedger + PublicationJournal
     ),
) {
    let run = seed(ledger);
    for variant in 0..4 {
        let mut changed = delta();
        match variant {
            0 => changed.engine_outputs[0].digest = super::digest(99),
            1 => changed.engine_outputs[0].target = "/outputs/foreign.json".into(),
            2 => {
                changed.engine_outputs[0].language_source =
                    vpt_domain::transcript::LanguageSource::Reported
            }
            _ => {
                let TranscriptionChange::Accept(accepted) = changed.transcription.as_mut().unwrap()
                else {
                    unreachable!()
                };
                accepted.transcript.segments[0].words[0].text = "different private text".into();
            }
        }
        assert!(matches!(
            ledger.commit_step(run, &changed, &fact()),
            HistoryWrite::Unconfirmed(_)
        ));
        assert!(ledger.engine_output(&artifact().target).unwrap().is_none());
        assert!(
            ledger
                .accepted_transcript(&fixture().recording)
                .unwrap()
                .is_none()
        );
        assert!(ledger.pending_publications().unwrap().is_empty());
        assert_eq!(ledger.pending_exports().unwrap().len(), 1);
    }
}

pub(crate) fn checker_output_is_retained_verbatim_with_the_record_output(
    ledger: &(
         impl RecordingLedger + RunHistory + EngineOutputLedger + TranscriptLedger + PublicationJournal
     ),
) {
    let run = seed(ledger);
    let mut changed = delta();
    let mut checker = artifact();
    checker.engine = "comparison".into();
    checker.language_source = vpt_domain::transcript::LanguageSource::Reported;
    checker.target = format!("/outputs/{}.comparison.json", checker.recording).into();
    let mut value: serde_json::Value = serde_json::from_slice(&checker.bytes).unwrap();
    value["engine"]["name"] = "comparison".into();
    value["diagnostic_extension"] = "private extension".into();
    checker.bytes = serde_json::to_vec(&value).unwrap();
    checker.digest = vpt_domain::digest::Sha256Digest(Sha256::digest(&checker.bytes).into());
    changed.engine_outputs.push(checker.clone());
    changed.commit.publications.push(DirtyPublication {
        target: checker.target.clone(),
        intended: checker.digest,
        expected_previous: None,
        recorded_at: checker.created_at,
    });
    let TranscriptionChange::Accept(accepted) = changed.transcription.as_mut().unwrap() else {
        unreachable!()
    };
    accepted.from_checker = false;
    let mut metadata = accepted.transcript.engine.clone();
    metadata.name = checker.engine.clone();
    accepted.checker_engine = Some(metadata);
    committed(ledger.commit_step(run, &changed, &fact()));
    let TranscriptionChange::Accept(expected) = changed.transcription.as_ref().unwrap() else {
        unreachable!()
    };
    assert_eq!(
        ledger
            .accepted_transcript(&expected.recording)
            .unwrap()
            .as_ref(),
        Some(expected.as_ref())
    );
    assert_eq!(
        ledger.engine_output(&checker.target).unwrap(),
        Some(checker)
    );
    assert_eq!(
        ledger.engine_output(&artifact().target).unwrap(),
        Some(artifact())
    );
}

macro_rules! cases {
    ($case:ident) => {
        $case!(checker_metadata_must_match_the_published_output);
        $case!(raw_bytes_and_publication_commit_with_accepted_state);
        $case!(missing_or_wrong_publication_refuses_raw_output);
        $case!(pending_publication_prevents_replacing_saved_engine_bytes);
        $case!(replacing_a_published_output_requires_its_saved_previous_digest);
        $case!(mismatched_bytes_identity_or_record_text_roll_back_the_batch);
        $case!(checker_output_is_retained_verbatim_with_the_record_output);
    };
}
pub(crate) use cases;
