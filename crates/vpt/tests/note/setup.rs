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
    let (sandbox, mut rows) = configured_with(&[7]);
    (sandbox, rows.remove(0))
}

pub(super) fn configured_with(seeds: &[u8]) -> (Sandbox, Vec<RecordingRecord>) {
    configured_saying(seeds, "um private words")
}

pub(super) fn configured_saying(seeds: &[u8], words: &str) -> (Sandbox, Vec<RecordingRecord>) {
    let sandbox = fixture::configured("note-write");
    let rows: Vec<_> = seeds
        .iter()
        .map(|seed| fixture::record(&sandbox, *seed, StageStates::fresh()))
        .collect();
    fixture::seed(&sandbox, rows.clone());
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    for row in &rows {
        accept(&ledger, row, words);
    }
    (sandbox, rows)
}

fn accept(ledger: &SqliteLedger, row: &RecordingRecord, words: &str) {
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
                speaker: None,
                start: 0.0,
                end: 1.0,
                text: words.into(),
                words: vec![Word {
                    text: words.into(),
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
