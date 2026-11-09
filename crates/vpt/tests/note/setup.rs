use super::{fixture, support};
use support::{Sandbox, stderr, stdout};
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, RunKind, WorkOutcome},
    time::UtcInstant,
    transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word},
};

pub(super) fn configured() -> (Sandbox, RecordingRecord) {
    let sandbox = fixture::configured("note-write");
    let row = fixture::record(&sandbox, 7, StageStates::fresh());
    fixture::seed(&sandbox, vec![row.clone()]);
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let at = UtcInstant { secs: 50 };
    let accepted = AcceptedTranscript {
        checker_engine: Some(EngineMetadata {
            name: "comparison".into(),
            tool: "checker".into(),
            version: "1".into(),
            model: "second".into(),
            family: "checker".into(),
        }),
        recording: row.id.clone(),
        requested_language: "en-US".into(),
        duration: 19.0,
        accepted_at: at,
        lexical_classification: false,
        from_checker: false,
        uncertainty: Default::default(),
        transcript: Transcript {
            engine: EngineMetadata {
                name: "fixture".into(),
                tool: "fixture".into(),
                version: "1".into(),
                model: "test".into(),
                family: "independent".into(),
            },
            language: "en-US".into(),
            language_source: LanguageSource::Requested,
            local: true,
            segments: vec![Segment {
                start: 0.0,
                end: 1.0,
                text: "um private words".into(),
                words: vec![Word {
                    text: "um private words".into(),
                    start: 0.0,
                    end: 1.0,
                    confidence: None,
                }],
            }],
        },
    };
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Transcribe, at) else {
        panic!("run")
    };
    assert!(matches!(
        ledger.commit_step(
            run.run,
            &WorkDelta {
                transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
                ..Default::default()
            },
            &[(
                at,
                HistoryFact::WorkStep {
                    recording: Some(row.id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    (sandbox, row)
}

pub(super) fn invoke(sandbox: &Sandbox, id: &str) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(["note", "write", id, "--json"]),
        std::time::Duration::from_secs(5),
    )
}
pub(super) fn success(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).unwrap()
}
pub(super) fn saved(sandbox: &Sandbox, row: &RecordingRecord) -> NoteArtifact {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap()
}
