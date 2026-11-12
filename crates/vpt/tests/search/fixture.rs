use super::*;
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, RunKind, WorkOutcome},
    time::UtcInstant,
    transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word},
    uncertainty::{Classification, Diagnostic},
};

pub(super) fn transcribed(sandbox: &Sandbox, seed: u8, segments: &[(f64, &str)]) -> RecordingId {
    let row = record::record(sandbox, seed, StageStates::fresh());
    let id = row.id.clone();
    let state = sandbox.path().join("state/vpt");
    if state.exists() {
        let ledger = SqliteLedger::open(&RootDir::open(&state).unwrap()).unwrap();
        ledger
            .commit(&LedgerCommit {
                recordings: vec![row],
                ..Default::default()
            })
            .unwrap();
    } else {
        record::seed(sandbox, vec![row]);
    }
    accept(sandbox, &id, segments);
    json(&invoke(sandbox, &["note", "write", id.as_str(), "--json"]));
    id
}

fn accept(sandbox: &Sandbox, id: &RecordingId, segments: &[(f64, &str)]) {
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let at = UtcInstant { secs: 100 };
    let accepted = AcceptedTranscript {
        checker_engine: None,
        recording: id.clone(),
        requested_language: "en-US".into(),
        duration: segments.last().map_or(1.0, |(start, _)| start + 5.0),
        accepted_at: at,
        lexical_classification: false,
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
            segments: segments
                .iter()
                .map(|(start, text)| Segment {
                    speaker: None,
                    start: *start,
                    end: start + 4.0,
                    text: (*text).into(),
                    words: text
                        .split(' ')
                        .map(|word| Word {
                            text: word.into(),
                            start: *start,
                            end: start + 4.0,
                            confidence: None,
                        })
                        .collect(),
                })
                .collect(),
        },
        uncertainty: Classification {
            lexical: vec![],
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
}

pub(super) fn analyzed(sandbox: &Sandbox, id: &RecordingId, proposal: &str) {
    let script = sandbox.path().join("agent.sh");
    let document = sandbox.path().join("proposal.json");
    std::fs::write(&script, "#!/bin/sh\n/bin/cat >/dev/null\n/bin/cat \"$1\"\n").unwrap();
    std::fs::write(&document, proposal).unwrap();
    record::write_config(
        sandbox,
        &format!("[analysis]\ncommand=['/bin/sh', {script:?}, {document:?}]\n"),
    );
    json(&invoke(sandbox, &["analyze", id.as_str(), "--json"]));
    std::fs::remove_file(script).unwrap();
    std::fs::remove_file(document).unwrap();
}
