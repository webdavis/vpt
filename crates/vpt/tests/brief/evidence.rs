use super::*;
use vpt_adapters::RootDir;
use vpt_domain::{
    run::{HistoryFact, RunKind, WorkOutcome},
    time::UtcInstant,
    transcript::{EngineMetadata, LanguageSource, Segment, Transcript, Word},
    uncertainty::{Classification, Diagnostic},
};
pub(super) fn seed(sandbox: &Sandbox) -> vpt_domain::identity::RecordingId {
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
            segments: vec![Segment {
                speaker: None,
                start: 0.0,
                end: 1.0,
                text: "Private accepted quote".into(),
                words: ["Private", "accepted", "quote"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, text)| Word {
                        text: text.into(),
                        start: i as f64 / 3.0,
                        end: (i + 1) as f64 / 3.0,
                        confidence: None,
                    })
                    .collect(),
            }],
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
    json(&invoke(sandbox, &["note", "write", id.as_str(), "--json"]));
    json(&invoke(
        sandbox,
        &["confirm", id.as_str(), "--tag", "planning", "--json"],
    ));
    id
}
#[test]
fn large_brief_counts_select_available_evidence_in_preview_and_publication() {
    let sandbox = fixture::configured("brief-large-counts");
    let id = seed(&sandbox);
    fixture::write_config(
        &sandbox,
        "[brief]\nmax_notes=4294967295\nmax_spans_per_note=4294967295\n",
    );
    for extra in [vec!["--dry-run"], vec![]] {
        let result = json(&invoke(&sandbox, &manual(&extra)));
        assert_eq!(result["items"].as_array().unwrap().len(), 1);
        assert_eq!(result["items"][0]["text"], "Private accepted quote");
        assert_eq!(result["items"][0]["sources"][0]["recording"], id.as_str());
    }
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let occasion = ledger.occasions().unwrap().remove(0);
    let artifact = ledger.brief(&occasion.id).unwrap().unwrap();
    assert_eq!(artifact.snapshot.pack.items.len(), 1);
    assert_eq!(
        std::fs::read_to_string(&artifact.target).unwrap(),
        artifact.text
    );
}

#[test]
fn brief_quotes_accepted_text_with_current_note_digest_and_saved_confirmed_tags() {
    let sandbox = fixture::configured("brief-evidence");
    let id = seed(&sandbox);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let note = ledger.note(&id, NoteStage::Transcript).unwrap().unwrap();
    std::fs::write(
        &note.target,
        format!("{}\nPrivate operator text\n", note.text),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let result = json(&invoke(&sandbox, &manual(&["--dry-run"])));
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["items"][0]["text"], "Private accepted quote");
    assert_eq!(result["items"][0]["certainty"], "unverified");
    assert_eq!(result["items"][0]["sources"][0]["recording"], id.as_str());
    assert_eq!(
        result["items"][0]["sources"][0]["note"],
        note.target.to_str().unwrap()
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    let written = json(&invoke(&sandbox, &manual(&[])));
    let occasion =
        vpt_domain::occasion::OccasionId::parse(written["occasion"]["id"].as_str().unwrap())
            .unwrap();
    let brief = ledger.brief(&occasion).unwrap().unwrap();
    let stores =
        vpt_adapters::FilesystemStores::open_read_only(&[note.target.parent().unwrap().into()])
            .unwrap();
    assert_eq!(
        brief.snapshot.pack.sources[0].digest,
        stores.read_note(&note.target).unwrap().unwrap().digest
    );
    assert!(!brief.text.contains("Private operator text"));
}
#[test]
fn expired_and_missing_transcript_files_do_not_supply_brief_evidence() {
    let sandbox = fixture::configured("brief-missing-evidence");
    let id = seed(&sandbox);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let note = ledger.note(&id, NoteStage::Transcript).unwrap().unwrap();
    std::fs::remove_file(&note.target).unwrap();
    let result = json(&invoke(&sandbox, &manual(&["--dry-run"])));
    assert!(result["items"].as_array().unwrap().is_empty());
    std::fs::write(&note.target, &note.text).unwrap();
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    connection
        .execute("UPDATE recordings SET stage_note='expired'", [])
        .unwrap();
    fixture::preserve_wal(&connection);
    drop(connection);
    let before = fixture::snapshot(&sandbox);
    let result = json(&invoke(&sandbox, &manual(&["--dry-run"])));
    assert!(result["items"].as_array().unwrap().is_empty());
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn participant_selection_uses_confirmed_terms_current_index_and_saved_mentions() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = fixture::configured("brief-participant");
    let id = seed(&sandbox);
    let terms = sandbox.config_path().with_file_name("known-terms.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path={terms:?}\n"),
    );
    std::fs::write(&terms, "Private\n").unwrap();
    std::fs::set_permissions(&terms, std::fs::Permissions::from_mode(0o600)).unwrap();
    let indexed = sandbox.path().join("home/.vpt/transcripts/Private.md");
    std::fs::write(&indexed, "# Private\n").unwrap();
    json(&invoke(&sandbox, &["note", "write", id.as_str(), "--json"]));
    let args = [
        "brief",
        "--title",
        "Planning",
        "--at",
        "2026-10-01T10:00:00Z",
        "--participant",
        "Private",
        "--explain",
        "--json",
    ];
    let before = fixture::snapshot(&sandbox);
    let result = json(&invoke(&sandbox, &args));
    assert_eq!(result["items"][0]["selector"], "participant");
    assert_eq!(
        result["occasion"]["participants"][0]["matched_note"],
        indexed.to_str().unwrap()
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    std::fs::write(
        sandbox.path().join("home/.vpt/analysis/Private.md"),
        "# Private\n",
    )
    .unwrap();
    let result = json(&invoke(&sandbox, &args));
    assert_eq!(
        result["occasion"]["participants"][0]["term"],
        "ambiguous-note"
    );
    assert!(result["items"].as_array().unwrap().is_empty());
}
#[test]
fn changed_managed_transcript_content_is_refused_without_writes_or_private_errors() {
    let sandbox = fixture::configured("brief-edited-source");
    let id = seed(&sandbox);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let note = ledger.note(&id, NoteStage::Transcript).unwrap().unwrap();
    std::fs::write(
        &note.target,
        note.text
            .replace("Private accepted quote", "Private unchecked replacement"),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &manual(&["--dry-run"]));
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}
#[test]
fn human_explain_lists_the_selected_candidate_and_matched_tag_without_writes() {
    let sandbox = fixture::configured("brief-explain");
    let id = seed(&sandbox);
    let before = fixture::snapshot(&sandbox);
    let mut args = manual(&["--explain"]);
    args.retain(|arg| *arg != "--json");
    let output = invoke(&sandbox, &args);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = support::stdout(&output);
    assert!(text.contains(&format!("{}: selected; tag;", id)), "{text}");
    assert!(text.contains("matched: planning"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn configured_zero_caps_preserve_omission_counts_and_confirmation_guidance() {
    let sandbox = fixture::configured("brief-cap-counts");
    let id = seed(&sandbox);
    for (setting, notes, spans) in [("max_notes=0", 1, 0), ("max_spans_per_note=0", 0, 1)] {
        fixture::write_config(&sandbox, &format!("[brief]\n{setting}\n"));
        let before = fixture::snapshot(&sandbox);
        let result = json(&invoke(&sandbox, &manual(&["--explain"])));
        assert!(result["items"].as_array().unwrap().is_empty());
        assert_eq!(result["selection"]["omitted_notes"], notes);
        assert_eq!(result["selection"]["omitted_spans"], spans);
        assert_eq!(
            result["selection"]["candidates"][0]["recording"],
            id.as_str()
        );
        assert!(
            result["not_selected"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["command"].as_str().is_some_and(|s| s
                    .starts_with("vpt confirm --term")
                    && s.contains("Private person")))
        );
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}
