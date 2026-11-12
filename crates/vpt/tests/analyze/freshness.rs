use super::*;
use vpt_adapters::RootDir;
use vpt_domain::{
    review::Resolution,
    run::{HistoryFact, RunKind, WorkOutcome},
    time::UtcInstant,
};

#[test]
fn lexical_review_invalidates_analysis_in_the_same_publication() {
    let (sandbox, record) = note::configured();
    let writer =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let mut transcript = writer.accepted_transcript(&record.id).unwrap().unwrap();
    transcript.lexical_classification = true;
    transcript.transcript.segments[0].words[0].confidence = Some(0.1);
    transcript
        .uncertainty
        .lexical
        .push(vpt_domain::uncertainty::LexicalFlag {
            class: vpt_domain::uncertainty::Class::LowConfidence,
            ranges: std::iter::once(0..1).collect(),
            record_text: "um private words".into(),
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
    drop(writer);
    agent(&sandbox, "");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let before = ledger(&sandbox)
        .note(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    assert!(before.text.contains("[built-on-flagged-text]") && before.text.contains("[unsourced]"));
    let lexical = ledger(&sandbox)
        .transcript_flags(&record.id)
        .unwrap()
        .remove(0);
    note::success(&support::run_with_budget(
        sandbox.vpt().args([
            "review",
            record.id.as_str(),
            "--resolve",
            &lexical.identifier.hex(),
            "--correct",
            "Fixed words",
            "--json",
        ]),
        Duration::from_secs(5),
    ));
    let reader = ledger(&sandbox);
    for stage in [NoteStage::Transcript, NoteStage::Analysis] {
        let note = reader.note(&record.id, stage).unwrap().unwrap();
        assert_eq!(fs::read_to_string(&note.target).unwrap(), note.text);
        assert!(
            !note.text.contains("[built-on-flagged-text]") && !note.text.contains("[unsourced]")
        );
        assert!(note.text.contains("vpt-open-flags: 0"));
        if stage == NoteStage::Transcript {
            assert!(note.text.contains("[corrected: Fixed words]"));
        }
    }
    let snapshot = reader.review_snapshot(&record.id).unwrap().unwrap();
    assert_eq!(snapshot.stale_findings.len(), 2);
    assert_eq!(
        reader.by_id(&record.id).unwrap().unwrap().stages.analysis,
        StageState::Pending
    );
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    let count: i64 = connection
        .query_row("SELECT open_flags FROM recordings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    drop(connection);
    drop(reader);
    let transcript = note::saved(&sandbox, &record);
    for _ in 0..2 {
        note::success(&support::run_with_budget(
            sandbox.vpt().args([
                "verify-note",
                transcript.target.to_str().unwrap(),
                "--recording",
                record.id.as_str(),
                "--json",
            ]),
            Duration::from_secs(5),
        ));
        let saved = note::saved(&sandbox, &record);
        assert!(
            saved.text.contains("[corrected: Fixed words]"),
            "verification must preserve lexical corrections"
        );
    }
    let finding = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap()
        .flags
        .into_iter()
        .find(|flag| {
            matches!(
                flag.content,
                vpt_domain::review::FlagContent::Verification(_)
            )
        })
        .unwrap();
    note::success(&support::run_with_budget(
        sandbox.vpt().args([
            "review",
            record.id.as_str(),
            "--resolve",
            &finding.identifier.hex(),
            "--correct",
            "Fixed words",
            "--json",
        ]),
        Duration::from_secs(5),
    ));
    for _ in 0..2 {
        note::success(&support::run_with_budget(
            sandbox.vpt().args([
                "verify-note",
                transcript.target.to_str().unwrap(),
                "--recording",
                record.id.as_str(),
                "--json",
            ]),
            Duration::from_secs(5),
        ));
        note::success(&note::invoke(&sandbox, record.id.as_str()));
        assert_eq!(
            note::saved(&sandbox, &record)
                .text
                .matches("[corrected: Fixed words]")
                .count(),
            2,
            "the lexical and finding corrections remain distinct even with identical text"
        );
        let current = ledger(&sandbox)
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap()
            .flags;
        let current = current
            .iter()
            .find(|flag| flag.identifier == finding.identifier)
            .unwrap();
        assert!(matches!(current.resolution, Resolution::Corrected { .. }));
    }
}

#[test]
fn changed_transcript_removes_stale_annotations_and_rechecking_keeps_the_review_decision() {
    for recheck in ["verify", "analyze"] {
        let (sandbox, record) = note::configured();
        agent(&sandbox, "");
        note::success(&invoke(&sandbox, record.id.as_str(), false));
        let flag = ledger(&sandbox)
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap()
            .flags
            .remove(0);
        note::success(&support::run_with_budget(
            sandbox.vpt().args([
                "review",
                record.id.as_str(),
                "--resolve",
                &flag.identifier.hex(),
                "--correct",
                "Operator correction",
                "--json",
            ]),
            Duration::from_secs(5),
        ));
        let writer =
            SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
        let mut transcript = writer.accepted_transcript(&record.id).unwrap().unwrap();
        let at = UtcInstant { secs: 300 };
        transcript.accepted_at = at;
        transcript.transcript.segments[0].words[0].start = 0.1;
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
        drop(writer);
        assert_eq!(
            ledger(&sandbox)
                .by_id(&record.id)
                .unwrap()
                .unwrap()
                .stages
                .analysis,
            StageState::Pending
        );
        note::success(&note::invoke(&sandbox, record.id.as_str()));
        let analysis = ledger(&sandbox)
            .note(&record.id, NoteStage::Analysis)
            .unwrap()
            .unwrap();
        assert!(
            !analysis.text.contains("[corrected:"),
            "stale findings must not be rendered as current"
        );
        assert!(analysis.text.contains("vpt-open-flags: 0"));
        assert!(
            ledger(&sandbox)
                .review_snapshot(&record.id)
                .unwrap()
                .unwrap()
                .flags
                .is_empty()
        );
        if recheck == "analyze" {
            note::success(&invoke(&sandbox, record.id.as_str(), false));
        } else {
            note::success(&support::run_with_budget(
                sandbox.vpt().args([
                    "verify-note",
                    analysis.target.to_str().unwrap(),
                    "--recording",
                    record.id.as_str(),
                    "--json",
                ]),
                Duration::from_secs(5),
            ));
        }
        let current = ledger(&sandbox)
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap();
        assert_eq!(current.flags[0].identifier, flag.identifier);
        assert!(matches!(
            current.flags[0].resolution,
            Resolution::Corrected { .. }
        ));
        let analysis = ledger(&sandbox)
            .note(&record.id, NoteStage::Analysis)
            .unwrap()
            .unwrap();
        assert!(analysis.text.contains("[corrected: Operator correction]"));
        assert!(analysis.text.contains("vpt-open-flags: 0"));
        note::success(&invoke(&sandbox, record.id.as_str(), false));
        assert!(
            ledger(&sandbox)
                .note(&record.id, NoteStage::Analysis)
                .unwrap()
                .unwrap()
                .text
                .contains("[corrected: Operator correction]")
        );
    }
}
