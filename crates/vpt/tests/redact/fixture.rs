use super::*;
use vpt_adapters::RootDir;
use vpt_domain::{
    run::{HistoryFact, RunKind, WorkOutcome},
    time::UtcInstant,
    transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word},
    uncertainty::{Classification, Diagnostic},
};
pub(super) fn seed(sandbox: &Sandbox) -> vpt_domain::identity::RecordingId {
    seed_flags(sandbox, false)
}
pub(super) fn seed_flags(sandbox: &Sandbox, flagged: bool) -> vpt_domain::identity::RecordingId {
    let row = fixture::record(sandbox, 9, StageStates::fresh());
    let id = row.id.clone();
    fixture::seed(sandbox, vec![row]);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let at = UtcInstant { secs: 100 };
    let accepted = AcceptedTranscript {
        checker_engine: None,
        recording: id.clone(),
        requested_language: "en-US".into(),
        duration: 19.0,
        accepted_at: at,
        lexical_classification: flagged,
        from_checker: false,
        transcript: Transcript {
            engine: EngineMetadata {
                name: "apple".into(),
                tool: "fixture".into(),
                version: "1".into(),
                model: "fixture".into(),
                family: "apple".into(),
            },
            language: "en-US".into(),
            language_source: LanguageSource::Reported,
            local: true,
            segments: vec![Segment {
                start: 0.0,
                end: 1.0,
                text: "Meet Alice with Bob via person@sample.test for 1234".into(),
                words: [
                    "Meet",
                    "Alice",
                    "with",
                    "Bob",
                    "via",
                    "person@sample.test",
                    "for",
                    "1234",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, text)| Word {
                    text: text.into(),
                    start: i as f64 / 8.0,
                    end: (i + 1) as f64 / 8.0,
                    confidence: None,
                })
                .collect(),
            }],
        },
        uncertainty: Classification {
            lexical: if flagged {
                vec![vpt_domain::uncertainty::LexicalFlag {
                    ranges: std::iter::once(7..8).collect(),
                    class: vpt_domain::uncertainty::Class::Numeric,
                    record_text: "1234".into(),
                    alternative_text: Some("5678".into()),
                    confidence: None,
                }]
            } else {
                vec![]
            },
            diagnostics: vec![Diagnostic::SingleEngine],
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
                    recording: Some(id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
    json(&invoke(sandbox, &["note", "write", id.as_str(), "--json"]));
    id
}
