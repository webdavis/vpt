use super::*;

pub(super) fn sql(sandbox: &Sandbox, sql: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(sql).unwrap();
}

#[test]
fn failed_agent_preserves_accepted_proposal_findings_and_analysis() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let before = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    for script in [
        "printf 'Private child output' >&2\nexit 29\n",
        "printf 'Private invalid JSON'\n",
    ] {
        fs::write(sandbox.path().join("agent.sh"), script).unwrap();
        let output = invoke(&sandbox, record.id.as_str(), false);
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private"));
        let after = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
        assert_eq!(after.proposal, before.proposal);
        assert_eq!(after.review.flags, before.review.flags);
        assert_eq!(after.record.stages.synthesis, StageState::Failed);
        let analysis = after
            .notes
            .iter()
            .find(|note| note.stage == NoteStage::Analysis)
            .unwrap();
        let old = before
            .notes
            .iter()
            .find(|note| note.stage == NoteStage::Analysis)
            .unwrap();
        assert_eq!(analysis.text, old.text);
        assert_eq!(fs::read_to_string(&analysis.target).unwrap(), old.text);
        assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    }
}

#[test]
fn transcript_edited_during_agent_execution_is_preserved_and_refused() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    note::success(&note::invoke(&sandbox, record.id.as_str()));
    let source = note::saved(&sandbox, &record);
    let replacement = vpt_adapters::note_document::Document::parse(&source.text)
        .unwrap()
        .replace_content("[00:00] Operator changed the transcript\n")
        .unwrap();
    fs::write(sandbox.path().join("captured.edited"), &replacement).unwrap();
    fs::write(
        sandbox.path().join("agent.sh"),
        "/bin/cat > \"$1\"\n/bin/cat \"$1.edited\" > \"$2\"\n/bin/cat \"$1.proposal\"\n",
    )
    .unwrap();
    let output = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["rule"], "synthesis_input_changed");
    assert!(
        ledger(&sandbox)
            .accepted_proposal(&record.id)
            .unwrap()
            .is_none()
    );
    assert!(
        ledger(&sandbox)
            .note(&record.id, NoteStage::Analysis)
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read_to_string(source.target).unwrap(), replacement);
}

#[test]
fn interrupted_publication_repairs_saved_analysis_without_rerunning_the_agent() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_analysis_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/analysis/%' BEGIN SELECT RAISE(ABORT, 'Private ack'); END",
    );
    let output = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([record.id.as_str()])
    );
    let before = ledger(&sandbox)
        .accepted_proposal(&record.id)
        .unwrap()
        .unwrap();
    assert!(!ledger(&sandbox).pending_publications().unwrap().is_empty());
    sql(&sandbox, "DROP TRIGGER refuse_analysis_ack");
    fs::write(
        sandbox.path().join("agent.sh"),
        "printf 'Unexpected agent rerun' >&2\nexit 29\n",
    )
    .unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(
        ledger(&sandbox).accepted_proposal(&record.id).unwrap(),
        Some(before)
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger(&sandbox)
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Succeeded
    );
}

#[test]
fn late_analysis_transaction_failure_rolls_back_proposal_findings_and_tags() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_analysis BEFORE INSERT ON owned_notes WHEN NEW.stage='analysis' BEGIN SELECT RAISE(ABORT, 'Private rollback'); END",
    );
    let output = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private"));
    let input = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    assert!(input.proposal.is_none());
    assert!(input.review.flags.is_empty());
    assert!(input.tags.is_none());
    assert!(
        !input
            .notes
            .iter()
            .any(|note| note.stage == NoteStage::Analysis)
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(input.record.stages.synthesis, StageState::Failed);
}

#[test]
fn synthesis_preserves_operator_edits_and_review_resolutions_on_repeated_claims() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let initial = ledger(&sandbox)
        .note(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    fs::write(
        &initial.target,
        initial.text.replace("tags: []", "tags: [operator]") + "\nOperator ending\n",
    )
    .unwrap();
    let flag = ledger(&sandbox)
        .review_snapshot(&record.id)
        .unwrap()
        .unwrap()
        .flags
        .remove(0);
    let output = support::run_with_budget(
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
    );
    note::success(&output);
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let next = ledger(&sandbox)
        .note(&record.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    assert_eq!(next.target, initial.target);
    assert_eq!(next.first_written, initial.first_written);
    assert!(next.text.ends_with("\nOperator ending\n"));
    assert!(next.text.contains("operator"));
    assert!(next.text.contains("[corrected: Operator correction]"));
    assert!(next.text.contains("vptOpenFlags: 0"));
    assert_eq!(
        ledger(&sandbox)
            .review_snapshot(&record.id)
            .unwrap()
            .unwrap()
            .flags[0]
            .identifier,
        flag.identifier
    );
}

#[test]
fn dry_run_with_pending_publication_performs_no_repair_or_agent_work() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_analysis_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/analysis/%' BEGIN SELECT RAISE(ABORT, 'Private ack'); END",
    );
    assert_eq!(
        invoke(&sandbox, record.id.as_str(), false).status.code(),
        Some(1)
    );
    let before = fixture::snapshot(&sandbox);
    note::success(&invoke(&sandbox, record.id.as_str(), true));
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert!(!ledger(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn agent_deadline_is_bounded_and_records_a_failed_attempt() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "timeout_secs = 1\n");
    fs::write(sandbox.path().join("agent.sh"), "exec /bin/sleep 10\n").unwrap();
    let started = std::time::Instant::now();
    let output = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("exceeded its deadline"));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(
        ledger(&sandbox)
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Failed
    );
    assert!(
        ledger(&sandbox)
            .accepted_proposal(&record.id)
            .unwrap()
            .is_none()
    );
}

#[test]
fn retry_after_transcript_ack_failure_uses_the_committed_proposal() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "");
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_transcript_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/transcripts/%' AND EXISTS(SELECT 1 FROM proposals) BEGIN SELECT RAISE(ABORT, 'Private ack'); END",
    );
    let output = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let before = ledger(&sandbox)
        .accepted_proposal(&record.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        ledger(&sandbox)
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .stages
            .synthesis,
        StageState::Succeeded
    );
    sql(&sandbox, "DROP TRIGGER refuse_transcript_ack");
    fs::write(sandbox.path().join("agent.sh"), "exit 29\n").unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert_eq!(
        ledger(&sandbox).accepted_proposal(&record.id).unwrap(),
        Some(before)
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
}
