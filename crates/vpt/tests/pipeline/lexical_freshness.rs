use super::*;
use vpt_domain::{
    run::{HistoryFact, RunKind, WorkOutcome},
    uncertainty::{Class, LexicalFlag},
};

pub(super) fn flag_first_word(sandbox: &Sandbox, record: &RecordingRecord) {
    let root = vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let writer = SqliteLedger::open(&root).unwrap();
    let mut transcript = writer.accepted_transcript(&record.id).unwrap().unwrap();
    transcript.lexical_classification = true;
    transcript.transcript.segments[0].words[0].confidence = Some(0.1);
    transcript.uncertainty.lexical.push(LexicalFlag {
        class: Class::LowConfidence,
        ranges: std::iter::once(0..1).collect(),
        record_text: "Private".into(),
        alternative_text: None,
        confidence: Some(0.1),
    });
    let at = transcript.accepted_at;
    let HistoryWrite::Committed(run) = writer.begin_run(RunKind::Transcribe, at) else {
        panic!("run")
    };
    assert!(matches!(
        writer.commit_step(
            run.run,
            &WorkDelta {
                transcription: Some(TranscriptionChange::Accept(Box::new(transcript))),
                ..Default::default()
            },
            &[(
                at,
                HistoryFact::WorkStep {
                    recording: Some(record.id.clone()),
                    outcome: WorkOutcome::Completed
                }
            )]
        ),
        HistoryWrite::Committed(_)
    ));
}

#[test]
fn lexical_confirmation_refreshes_inherited_uncertainty_once_without_retranscribing() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let record = ledger(&sandbox).recordings().unwrap().remove(0);
    flag_first_word(&sandbox, &record);
    analysis::enable(&sandbox);
    json(&invoke(&sandbox, &[]));
    let before = ledger(&sandbox)
        .accepted_proposal(&record.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        before.proposal.summary[0].grounding.inherited_flags.len(),
        1
    );
    let analysis = ledger(&sandbox)
        .note(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    assert!(analysis.text.contains("[built-on-flagged-text]"));
    let lexical = ledger(&sandbox)
        .transcript_flags(&record.id)
        .unwrap()
        .into_iter()
        .find(|flag| {
            before.proposal.summary[0]
                .grounding
                .inherited_flags
                .contains(&flag.identifier)
        })
        .unwrap();
    json(&support::run_with_budget(
        sandbox.vpt().args([
            "review",
            record.id.as_str(),
            "--resolve",
            &lexical.identifier.hex(),
            "--confirm",
            "--json",
        ]),
        std::time::Duration::from_secs(5),
    ));
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    assert_eq!(input.record.stages.analysis, StageState::Pending);
    assert!(
        !input
            .notes
            .iter()
            .find(|n| n.stage == NoteStage::Analysis)
            .unwrap()
            .text
            .contains("[built-on-flagged-text]")
    );
    assert!(!input.review.stale_findings.is_empty());
    let snapshot = fixture::snapshot(&sandbox);
    json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(fixture::snapshot(&sandbox), snapshot);
    let refreshed = json(&invoke(&sandbox, &[]));
    assert_eq!(
        refreshed["analyzed"],
        serde_json::json!([record.id.as_str()])
    );
    let current = ledger(&sandbox)
        .accepted_proposal(&record.id)
        .unwrap()
        .unwrap();
    assert_eq!(current.basis.transcript, before.basis.transcript);
    assert_ne!(current.basis.lexical_flags, before.basis.lexical_flags);
    assert_ne!(current.input_digest, before.input_digest);
    assert!(
        current.proposal.summary[0]
            .grounding
            .inherited_flags
            .is_empty()
    );
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    assert!(input.review.stale_findings.is_empty());
    assert!(input.review.stale_artifacts.is_empty());
    for note in &input.notes {
        assert!(!note.text.contains("[built-on-flagged-text]"));
        assert!(note.text.contains("vpt-open-flags: 0\n"));
    }
    let transcript = input
        .notes
        .iter()
        .find(|n| n.stage == NoteStage::Transcript)
        .unwrap();
    fs::write(
        &transcript.target,
        format!("{}\nOperator prose\n", transcript.text),
    )
    .unwrap();
    for _ in 0..2 {
        assert_eq!(
            json(&invoke(&sandbox, &[]))["analyzed"],
            serde_json::json!([])
        );
    }
    assert_eq!(analysis::calls(&sandbox, "calls"), 1);
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 2);
}
