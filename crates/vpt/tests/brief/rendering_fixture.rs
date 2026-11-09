use super::*;
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, RunKind, WorkOutcome},
    transcript::{Segment, Word},
    uncertainty::{Class, LexicalFlag},
};

pub(super) const CONFIRMED: &str = "Exact  quoted words.";
pub(super) const UNCERTAIN: &str = "Uncertain value.";

pub(super) fn prepared(profile: &str, style: &str) -> (Sandbox, RecordingId) {
    let sandbox = fixture::configured("brief-render-acceptance");
    let root = sandbox.path().join("home/.vpt");
    std::fs::create_dir_all(&root).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[note]\nprofile={profile:?}\nlink_style={style:?}\n[note.obsidian]\nvault_root={root:?}\nhub_briefs='Planning hub'\n"
        ),
    );
    let id = evidence::seed(&sandbox);
    let root = vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let mut accepted = ledger.accepted_transcript(&id).unwrap().unwrap();
    accepted.lexical_classification = true;
    accepted.uncertainty.diagnostics.clear();
    accepted.transcript.segments = vec![
        segment(CONFIRMED, 0.25, &["Exact", "quoted", "words."]),
        segment(UNCERTAIN, 2.25, &["Uncertain", "value."]),
    ];
    accepted.transcript.segments[1].words[0].confidence = Some(0.1);
    accepted.uncertainty.lexical = vec![LexicalFlag {
        class: Class::LowConfidence,
        ranges: std::iter::once(3..4).collect(),
        record_text: "Uncertain".into(),
        alternative_text: None,
        confidence: Some(0.1),
    }];
    let at = accepted.accepted_at;
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Transcribe, at) else {
        panic!("run")
    };
    let saved = ledger.commit_step(
        run.run,
        &WorkDelta {
            transcription: Some(TranscriptionChange::Accept(Box::new(accepted))),
            ..Default::default()
        },
        &[(
            at,
            HistoryFact::WorkStep {
                recording: Some(id.clone()),
                outcome: WorkOutcome::Completed,
            },
        )],
    );
    assert!(matches!(saved, HistoryWrite::Committed(_)), "{saved:?}");
    drop(ledger);
    json(&invoke(&sandbox, &["note", "write", id.as_str(), "--json"]));
    (sandbox, id)
}

fn segment(text: &str, start: f64, words: &[&str]) -> Segment {
    Segment {
        text: text.into(),
        start,
        end: start + 1.0,
        words: words
            .iter()
            .enumerate()
            .map(|(index, word)| Word {
                text: (*word).into(),
                start: start + index as f64 / words.len() as f64,
                end: start + (index + 1) as f64 / words.len() as f64,
                confidence: Some(1.0),
            })
            .collect(),
    }
}

pub(super) fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}
