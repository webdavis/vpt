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
    let flags = match flagged {
        true => vec![vpt_domain::uncertainty::LexicalFlag {
            ranges: std::iter::once(7..8).collect(),
            class: vpt_domain::uncertainty::Class::Numeric,
            record_text: "1234".into(),
            alternative_text: Some("5678".into()),
            confidence: None,
        }],
        false => vec![],
    };
    seed_segments(
        sandbox,
        flags,
        &["Meet Alice with Bob via person@sample.test for 1234"],
    )
}
pub(super) fn flag_word(index: usize, text: &str) -> vpt_domain::uncertainty::LexicalFlag {
    vpt_domain::uncertainty::LexicalFlag {
        ranges: std::iter::once(index..index + 1).collect(),
        class: vpt_domain::uncertainty::Class::AgreedUnverified,
        record_text: text.into(),
        alternative_text: None,
        confidence: None,
    }
}
pub(super) fn seed_segments(
    sandbox: &Sandbox,
    flags: Vec<vpt_domain::uncertainty::LexicalFlag>,
    texts: &[&str],
) -> vpt_domain::identity::RecordingId {
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
        lexical_classification: !flags.is_empty(),
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
            segments: texts
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    let start = index as f64 * 2.0;
                    let words: Vec<_> = text.split_whitespace().collect();
                    let step = 1.0 / words.len() as f64;
                    Segment {
                        speaker: None,
                        start,
                        end: start + 1.0,
                        text: (*text).into(),
                        words: words
                            .iter()
                            .enumerate()
                            .map(|(i, word)| Word {
                                text: (*word).into(),
                                start: start + i as f64 * step,
                                end: start + (i + 1) as f64 * step,
                                confidence: None,
                            })
                            .collect(),
                    }
                })
                .collect(),
        },
        uncertainty: Classification {
            lexical: flags,
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
