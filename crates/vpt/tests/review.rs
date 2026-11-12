#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#![allow(clippy::single_range_in_vec_init, reason = "Flags contain word ranges")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "review/output.rs"]
mod output;
mod support;

use support::{Sandbox, run, stderr, stdout};
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, RunKind, WorkOutcome},
    transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word},
    uncertainty::{Class, Classification, Diagnostic, LexicalFlag},
};

fn configured() -> (Sandbox, vpt_domain::identity::RecordingId) {
    let sandbox = fixture::configured("review");
    let row = fixture::record(&sandbox, 9, StageStates::fresh());
    let id = row.id.clone();
    fixture::seed(&sandbox, vec![row]);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let words: Vec<_> = [
        "we",
        "paid",
        "42",
        "to",
        "SecretAlice",
        "then",
        "SecretAlice",
        "um",
        "the",
    ]
    .iter()
    .enumerate()
    .map(|(i, text)| Word {
        text: text.to_string(),
        start: i as f64 + 0.25,
        end: i as f64 + 1.0,
        confidence: (i == 7).then_some(0.1),
    })
    .collect();
    let flag = |class, ranges, text: &str, alt: Option<&str>, confidence| LexicalFlag {
        class,
        ranges,
        record_text: text.into(),
        alternative_text: alt.map(str::to_owned),
        confidence,
    };
    let at = vpt_domain::time::UtcInstant { secs: 60 };
    let accepted = AcceptedTranscript {
        checker_engine: None,
        recording: id.clone(),
        requested_language: "en-US".into(),
        duration: 9.0,
        accepted_at: at,
        lexical_classification: true,
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
                speaker: None,
                start: 0.0,
                end: 9.0,
                text: words
                    .iter()
                    .map(|word| word.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                words,
            }],
        },
        uncertainty: Classification {
            lexical: vec![
                flag(
                    Class::AgreedUnverified,
                    vec![4..5, 6..7],
                    "SecretAlice",
                    None,
                    None,
                ),
                flag(Class::Formatting, vec![8..9], "the", Some(""), None),
                flag(Class::LowConfidence, vec![7..8], "um", None, Some(0.1)),
                flag(Class::Numeric, vec![2..3], "42", Some("43"), None),
            ],
            diagnostics: vec![Diagnostic::SingleEngine],
        },
    };
    let HistoryWrite::Committed(start) = ledger.begin_run(RunKind::Transcribe, at) else {
        panic!("run")
    };
    assert!(matches!(
        ledger.commit_step(
            start.run,
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
    (sandbox, id)
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).unwrap()
}

#[test]
fn review_returns_ranked_flags_occurrences_and_separate_diagnostics_without_writes() {
    let (sandbox, id) = configured();
    let before = fixture::snapshot(&sandbox);
    let result = json(&run(sandbox.vpt().args(["review", id.as_str(), "--json"])));
    assert_eq!(result["command"], "review");
    assert_eq!(result["id"], id.as_str());
    assert_eq!(result["open_flags"], 3);
    assert_eq!(result["flags"][0]["class"], "numeric");
    assert_eq!(result["flags"][1]["class"], "low-confidence");
    assert_eq!(result["flags"][2]["class"], "agreed-unverified");
    assert_eq!(result["flags"][0]["record_text"], "42");
    assert_eq!(result["flags"][0]["alternative_text"], "43");
    assert_eq!(result["flags"][0]["timecode"], "00:02");
    assert_eq!(result["flags"][2]["occurrence_count"], 2);
    assert_eq!(result["flags"][2]["occurrences"][1]["start"], 6.25);
    assert_eq!(result["flags"][2]["occurrences"][1]["end"], 7.0);
    assert_eq!(result["flags"][0]["identifier"].as_str().unwrap().len(), 64);
    assert_eq!(result["diagnostics"][0]["class"], "single-engine");
    assert!(result["diagnostics"][0].get("occurrences").is_none());
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn human_review_exposes_text_alternatives_and_first_timecodes() {
    let (sandbox, id) = configured();
    let output = run(sandbox.vpt().args(["review", id.as_str()]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("[00:02] numeric"));
    assert!(text.contains("\"42\" | \"43\""));
    assert!(text.contains("SecretAlice"));
    assert!(text.contains("2 occurrences"));
    assert!(text.contains("single-engine"));
    assert!(!text.contains("formatting"));
}

#[test]
fn absent_or_invalid_recording_is_a_usage_error_without_creating_state() {
    let sandbox = fixture::configured("review-absent");
    let before = fixture::snapshot(&sandbox);
    for id in ["bad-id", "2026-08-24T144736-4f3ab19c02de"] {
        let output = run(sandbox.vpt().args(["review", id, "--json"]));
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
        assert_eq!(error["error"]["kind"], "usage");
        assert!(!stderr(&output).contains("unknown verb"));
    }
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn an_ingested_recording_without_accepted_text_has_an_empty_review() {
    let sandbox = fixture::configured("review-untranscribed");
    let row = fixture::record(&sandbox, 8, StageStates::fresh());
    let id = row.id.clone();
    fixture::seed(&sandbox, vec![row]);
    let before = fixture::snapshot(&sandbox);
    let result = json(&run(sandbox.vpt().args(["review", id.as_str(), "--json"])));
    assert_eq!(result["transcribed"], false);
    assert_eq!(result["open_flags"], 0);
    assert_eq!(result["flags"], serde_json::json!([]));
    assert_eq!(result["diagnostics"], serde_json::json!([]));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[path = "review/resolution.rs"]
mod resolution;

#[path = "review/mentions.rs"]
mod mentions;
#[path = "review/workflow.rs"]
mod workflow;
