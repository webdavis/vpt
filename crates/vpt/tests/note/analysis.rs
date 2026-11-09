use super::*;
use std::fs;
use vpt_adapters::{FilesystemStores, note_document::Document};
use vpt_domain::{
    grounding::Grounder,
    proposal::{Claim, Proposal},
};

fn reader(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}
fn analysis(sandbox: &Sandbox, row: &RecordingRecord) -> NoteArtifact {
    reader(sandbox)
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap()
}
fn seeded(extra: &str) -> (Sandbox, RecordingRecord) {
    let (sandbox, row) = configured();
    fixture::write_config(&sandbox, extra);
    success(&invoke(&sandbox, row.id.as_str()));
    let source = saved(&sandbox, &row);
    let ledger =
        SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap()).unwrap();
    let transcript = ledger.accepted_transcript(&row.id).unwrap().unwrap();
    let flags = ledger.transcript_flags(&row.id).unwrap();
    let checked = Grounder::new(
        transcript.transcript.segments.iter().flat_map(|s| &s.words),
        &flags,
        transcript.duration,
        15,
    )
    .unwrap()
    .proposal(Proposal {
        by: "private agent".into(),
        summary: vec![Claim {
            text: "private words".into(),
            source_references: vec!["00:00-00:01".into()],
        }],
        actions: vec![Claim {
            text: "another task".into(),
            source_references: vec![],
        }],
        tags: vec![],
        relations: vec![],
    })
    .unwrap();
    let content = vpt_domain::note::analysis_content(&checked).unwrap();
    let mut note = source.clone();
    note.stage = NoteStage::Analysis;
    note.target = sandbox.path().join("home/.vpt/analysis/analysis.md");
    note.text = Document::parse(&source.text)
        .unwrap()
        .replace_content(&content)
        .unwrap()
        .replace("vptStage: \"transcript\"", "vptStage: \"analysis\"");
    let stores = FilesystemStores::open(&[]).unwrap();
    note.digest = stores.digest_bytes(note.text.as_bytes());
    note.content_digest = stores.digest_bytes(content.as_bytes());
    let at = UtcInstant { secs: 100 };
    note.updated_at = at;
    note.first_written = at;
    let accepted = AcceptedProposal {
        vocabulary_publication: None,
        recording: row.id.clone(),
        input_digest: source.content_digest,
        basis: ledger.grounding_basis(&row.id).unwrap().unwrap(),
        grounding_window_secs: 15,
        proposal: checked,
        accepted_at: at,
    };
    let HistoryWrite::Committed(run) = ledger.begin_run(RunKind::Note, at) else {
        panic!("run")
    };
    let delta = WorkDelta {
        proposal: Some(ProposalChange::Accept(Box::new(accepted))),
        notes: vec![NoteChange {
            artifact: note.clone(),
            expected_saved: None,
            relocated_from: None,
        }],
        commit: LedgerCommit {
            publications: vec![DirtyPublication {
                target: note.target.clone(),
                expected_previous: None,
                intended: note.digest,
                recorded_at: at,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let outcome = ledger.commit_step(
        run.run,
        &delta,
        &[(
            at,
            HistoryFact::WorkStep {
                recording: Some(row.id.clone()),
                outcome: WorkOutcome::Completed,
            },
        )],
    );
    assert!(matches!(outcome, HistoryWrite::Committed(_)), "{outcome:?}");
    fs::write(&note.target, &note.text).unwrap();
    ledger.clear_publication(&note.target).unwrap();
    (sandbox, row)
}

#[test]
fn both_notes_rewrite_with_reciprocal_links_and_preserve_operator_prose() {
    for extra in ["", "[note]\nlink_style = 'wiki'\n"] {
        let (sandbox, row) = seeded(extra);
        let before = analysis(&sandbox, &row);
        for note in [saved(&sandbox, &row), before.clone()] {
            fs::write(&note.target, format!("{}\nOperator ending\n", note.text)).unwrap();
        }
        success(&invoke(&sandbox, row.id.as_str()));
        let transcript = saved(&sandbox, &row);
        let next = analysis(&sandbox, &row);
        assert!(transcript.text.contains("- Analysis: "));
        assert!(next.text.contains("- Transcript: "));
        assert_eq!(
            Document::parse(&next.text).unwrap().content().unwrap(),
            Document::parse(&before.text).unwrap().content().unwrap()
        );
        for note in [&transcript, &next] {
            assert!(note.text.ends_with("Operator ending\n"));
            assert_eq!(fs::read_to_string(&note.target).unwrap(), note.text);
        }
        assert!(reader(&sandbox).pending_publications().unwrap().is_empty());
        success(&invoke(&sandbox, row.id.as_str()));
        assert_eq!(analysis(&sandbox, &row).text, next.text);
    }
}

fn command(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(args).arg("--json"),
        std::time::Duration::from_secs(5),
    )
}
fn sql(sandbox: &Sandbox, statement: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(statement).unwrap();
}

#[test]
fn tags_update_both_notes_and_expired_analysis_requires_explicit_regeneration() {
    let (sandbox, row) = seeded("");
    success(&command(
        &sandbox,
        &["confirm", row.id.as_str(), "--tag", "project"],
    ));
    for note in [saved(&sandbox, &row), analysis(&sandbox, &row)] {
        assert!(
            Document::parse(&note.text)
                .unwrap()
                .tags
                .contains(&"project".into())
        );
    }
    let old = analysis(&sandbox, &row);
    sql(&sandbox, "UPDATE recordings SET stage_synthesis='expired'");
    fs::remove_file(&old.target).unwrap();
    success(&command(
        &sandbox,
        &["confirm", row.id.as_str(), "--tag", "later"],
    ));
    assert!(!old.target.exists());
    assert_eq!(analysis(&sandbox, &row), old);
    assert_eq!(
        reader(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Expired
    );
    success(&invoke(&sandbox, row.id.as_str()));
    let next = analysis(&sandbox, &row);
    assert_eq!(fs::read_to_string(&next.target).unwrap(), next.text);
    assert!(
        Document::parse(&next.text)
            .unwrap()
            .tags
            .contains(&"later".into())
    );
}

#[test]
fn renamed_analysis_updates_both_links_and_invalid_analysis_refuses_the_whole_change() {
    let (sandbox, row) = seeded("");
    let old = analysis(&sandbox, &row);
    let renamed = old.target.with_file_name("renamed analysis.md");
    fs::rename(&old.target, &renamed).unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    assert_eq!(analysis(&sandbox, &row).target, renamed);
    assert!(
        saved(&sandbox, &row)
            .text
            .contains("../analysis/renamed%20analysis.md")
    );
    let before = saved(&sandbox, &row);
    let invalid = fs::read_to_string(&renamed)
        .unwrap()
        .replace("<!-- vpt:content end -->", "");
    fs::write(&renamed, &invalid).unwrap();
    let output = command(&sandbox, &["confirm", row.id.as_str(), "--tag", "refused"]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(saved(&sandbox, &row), before);
    assert_eq!(fs::read_to_string(&before.target).unwrap(), before.text);
    assert_eq!(fs::read_to_string(&renamed).unwrap(), invalid);
    assert!(reader(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn resolving_an_analysis_finding_updates_its_annotation_and_both_counts() {
    let (sandbox, row) = seeded("");
    let note = analysis(&sandbox, &row);
    success(&command(
        &sandbox,
        &[
            "verify-note",
            note.target.to_str().unwrap(),
            "--recording",
            row.id.as_str(),
        ],
    ));
    let flags = reader(&sandbox)
        .review_snapshot(&row.id)
        .unwrap()
        .unwrap()
        .flags;
    assert_eq!(flags.len(), 1);
    success(&command(
        &sandbox,
        &[
            "review",
            row.id.as_str(),
            "--resolve",
            &flags[0].identifier.hex(),
            "--correct",
            "operator] correction",
        ],
    ));
    let next = analysis(&sandbox, &row);
    assert!(
        next.text.contains("[corrected: operator&#93; correction]"),
        "{}",
        next.text
    );
    for note in [saved(&sandbox, &row), next.clone()] {
        assert!(note.text.contains("vptOpenFlags: 0\n"));
    }
    success(&invoke(&sandbox, row.id.as_str()));
    assert_eq!(analysis(&sandbox, &row).text, next.text);
}

#[test]
fn changed_transcript_content_keeps_synthesis_pending_after_analysis_metadata_publication() {
    let (sandbox, row) = seeded("");
    fixture::write_config(&sandbox, "[readability]\nenabled = true\n");
    success(&invoke(&sandbox, row.id.as_str()));
    let inputs = reader(&sandbox).note_inputs(&row.id).unwrap().unwrap();
    assert_ne!(
        inputs.proposal.unwrap().input_digest,
        saved(&sandbox, &row).content_digest
    );
    assert_eq!(inputs.record.stages.synthesis, StageState::Pending);
}

#[test]
fn pipeline_detects_an_analysis_only_change_and_dry_run_stays_observational() {
    let (sandbox, row) = seeded("");
    let source = sandbox.path().join("voice-memos/Recordings");
    fs::create_dir_all(&source).unwrap();
    fixture::write_config(
        &sandbox,
        &format!("[source]\nrecordings_dir = {source:?}\nread_titles = false\n"),
    );
    success(&invoke(&sandbox, row.id.as_str()));
    let note = analysis(&sandbox, &row);
    fs::write(
        &note.target,
        format!("{}\nOperator analysis ending\n", note.text),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let planned = success(&command(&sandbox, &["run", "--dry-run"]));
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert_eq!(planned["plan"][0]["stage"], "note");
    let result = success(&command(&sandbox, &["run"]));
    assert_eq!(result["notes"].as_array().unwrap().len(), 1);
    assert!(
        analysis(&sandbox, &row)
            .text
            .ends_with("Operator analysis ending\n")
    );
}

#[test]
fn analysis_transaction_failure_rolls_back_both_note_changes_and_the_tag() {
    let (sandbox, row) = seeded("");
    let transcript = saved(&sandbox, &row);
    let old_analysis = analysis(&sandbox, &row);
    sql(
        &sandbox,
        "CREATE TRIGGER reject_analysis BEFORE UPDATE ON owned_notes WHEN NEW.stage='analysis' BEGIN SELECT RAISE(ABORT, 'private canary'); END",
    );
    let output = command(&sandbox, &["confirm", row.id.as_str(), "--tag", "rejected"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("private canary"));
    assert!(output.stdout.is_empty());
    assert_eq!(saved(&sandbox, &row), transcript);
    assert_eq!(analysis(&sandbox, &row), old_analysis);
    for note in [transcript, old_analysis] {
        assert_eq!(fs::read_to_string(note.target).unwrap(), note.text);
    }
    assert!(reader(&sandbox).tags(&row.id).unwrap().is_none());
    assert!(reader(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn tag_confirmation_updates_live_analysis_without_regenerating_expired_transcript() {
    let (sandbox, row) = seeded("");
    let old = saved(&sandbox, &row);
    sql(&sandbox, "UPDATE recordings SET stage_note='expired'");
    fs::remove_file(&old.target).unwrap();
    success(&command(
        &sandbox,
        &["confirm", row.id.as_str(), "--tag", "analysis-only"],
    ));
    assert!(!old.target.exists());
    assert_eq!(saved(&sandbox, &row), old);
    assert!(
        Document::parse(&analysis(&sandbox, &row).text)
            .unwrap()
            .tags
            .contains(&"analysis-only".into())
    );
    assert_eq!(
        reader(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Expired
    );
}

#[test]
fn relation_confirmation_updates_both_notes_without_changing_external_targets() {
    let (sandbox, row) = seeded("");
    let target = sandbox.path().join("home/.vpt/briefs/Private target.md");
    let external = "---\naliases: [Alias]\n---\nPrivate external prose";
    fs::write(&target, external).unwrap();
    let before = [saved(&sandbox, &row), analysis(&sandbox, &row)];
    let output = command(
        &sandbox,
        &["confirm", row.id.as_str(), "--relation", "related:Alias"],
    );
    success(&output);
    assert!(!stdout(&output).contains("Private"));
    for (previous, next) in before
        .iter()
        .zip([saved(&sandbox, &row), analysis(&sandbox, &row)])
    {
        assert!(next.text.contains(
            "- Related: [Private target](../briefs/Private%20target.md) (confirmed manually)"
        ));
        assert_eq!(previous.content_digest, next.content_digest);
    }
    assert_eq!(fs::read_to_string(target).unwrap(), external);
}

#[test]
fn resolving_a_finding_with_both_notes_expired_keeps_them_expired() {
    let (sandbox, row) = seeded("");
    let note = analysis(&sandbox, &row);
    success(&command(
        &sandbox,
        &[
            "verify-note",
            note.target.to_str().unwrap(),
            "--recording",
            row.id.as_str(),
        ],
    ));
    let flag = reader(&sandbox)
        .review_snapshot(&row.id)
        .unwrap()
        .unwrap()
        .flags
        .remove(0);
    sql(
        &sandbox,
        "UPDATE recordings SET stage_note='expired', stage_synthesis='expired'",
    );
    let before = [saved(&sandbox, &row), analysis(&sandbox, &row)];
    for note in &before {
        fs::remove_file(&note.target).unwrap();
    }
    let result = success(&command(
        &sandbox,
        &[
            "review",
            row.id.as_str(),
            "--resolve",
            &flag.identifier.hex(),
            "--dismiss",
        ],
    ));
    assert!(result["resolved"]["path"].is_null());
    for note in &before {
        assert!(!note.target.exists());
    }
    let inputs = reader(&sandbox).note_inputs(&row.id).unwrap().unwrap();
    assert_eq!(inputs.notes, before);
    assert_eq!(inputs.record.stages.note, StageState::Expired);
    assert_eq!(inputs.record.stages.synthesis, StageState::Expired);
    assert!(matches!(
        inputs.review.flags[0].resolution,
        vpt_domain::review::Resolution::Dismissed { .. }
    ));
}
