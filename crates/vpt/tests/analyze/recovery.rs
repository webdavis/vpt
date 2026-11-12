use super::*;

fn pending_analysis() -> (Sandbox, RecordingRecord, std::path::PathBuf) {
    let (sandbox, record) = note::configured();
    let known = sandbox.path().join("zz-vocabulary/known-tags.txt");
    fs::create_dir_all(known.parent().unwrap()).unwrap();
    agent(
        &sandbox,
        &format!("[tags]\nnew_tags = 'write'\nknown_tags_path = {known:?}\n"),
    );
    let path = sandbox.path().join("captured.proposal");
    let mut proposal: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    proposal["tags"] = serde_json::json!([{"tag":"Private topic","reason":"private reason"}]);
    fs::write(path, proposal.to_string()).unwrap();
    super::workflow::sql(
        &sandbox,
        "CREATE TRIGGER refuse_vocabulary_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/known-tags.txt' BEGIN SELECT RAISE(ABORT, 'private ack'); END",
    );
    let failed = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(failed.status.code(), Some(1), "{}", stderr(&failed));
    let error: serde_json::Value = serde_json::from_str(&stderr(&failed)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([record.id.as_str()])
    );
    let before = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    assert_eq!(before.record.stages.analysis, StageState::Succeeded);
    let pending = ledger(&sandbox).pending_publications().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].target, known);
    for note in &before.notes {
        assert_eq!(fs::read_to_string(&note.target).unwrap(), note.text);
    }
    (sandbox, record, known)
}

#[test]
fn vocabulary_only_recovery_preserves_accepted_analysis_without_rerunning_the_agent() {
    let (sandbox, record, known) = pending_analysis();
    let before = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    let snapshot = fixture::snapshot(&sandbox);
    note::success(&invoke(&sandbox, record.id.as_str(), true));
    assert_eq!(fixture::snapshot(&sandbox), snapshot);
    super::workflow::sql(&sandbox, "DROP TRIGGER refuse_vocabulary_ack");
    fs::write(sandbox.path().join("agent.sh"), "exit 29\n").unwrap();
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    let after = ledger(&sandbox).note_inputs(&record.id).unwrap().unwrap();
    assert_eq!(after.proposal, before.proposal);
    assert_eq!(after.notes, before.notes);
    assert_eq!(after.record.stages.analysis, StageState::Succeeded);
    assert_eq!(fs::read_to_string(known).unwrap(), "private-topic\n");
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("private-topic"));
    assert!(
        !events.contains("review_needed"),
        "recovery must not send analysis alerts"
    );
}

#[test]
fn vocabulary_recovery_does_not_skip_analysis_after_grounding_settings_change() {
    let (sandbox, record, known) = pending_analysis();
    super::workflow::sql(&sandbox, "DROP TRIGGER refuse_vocabulary_ack");
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(
        sandbox.config_path(),
        config.replace("[analysis]", "[analysis]\ngrounding_window_secs = 1"),
    )
    .unwrap();
    let marker = sandbox.path().join("rerun");
    fs::write(
        sandbox.path().join("agent.sh"),
        format!("printf attempted > {marker:?}\nexit 29\n"),
    )
    .unwrap();
    let failed = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(failed.status.code(), Some(1), "{}", stderr(&failed));
    assert!(marker.exists(), "changed inputs still need a new proposal");
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(fs::read_to_string(known).unwrap(), "private-topic\n");
    assert_eq!(
        ledger(&sandbox)
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .stages
            .analysis,
        StageState::Failed
    );
}

#[test]
fn unrelated_vocabulary_recovery_does_not_skip_an_explicit_analysis_request() {
    let (sandbox, record, known) = pending_analysis();
    super::workflow::sql(&sandbox, "DROP TRIGGER refuse_vocabulary_ack");
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    super::workflow::sql(
        &sandbox,
        "CREATE TRIGGER refuse_vocabulary_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/known-tags.txt' BEGIN SELECT RAISE(ABORT, 'private ack'); END",
    );
    let failed = support::run_with_budget(
        sandbox.vpt().args([
            "confirm",
            record.id.as_str(),
            "--tag",
            "another-topic",
            "--json",
        ]),
        Duration::from_secs(5),
    );
    assert_eq!(failed.status.code(), Some(1), "{}", stderr(&failed));
    let pending = ledger(&sandbox).pending_publications().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].target, known);
    let accepted = ledger(&sandbox)
        .accepted_proposal(&record.id)
        .unwrap()
        .unwrap();
    assert_ne!(accepted.vocabulary_publication.as_ref(), Some(&pending[0]));
    super::workflow::sql(&sandbox, "DROP TRIGGER refuse_vocabulary_ack");
    fs::write(sandbox.path().join("agent.sh"), "exit 29\n").unwrap();
    let failed = invoke(&sandbox, record.id.as_str(), false);
    assert_eq!(failed.status.code(), Some(1), "{}", stderr(&failed));
    assert!(stderr(&failed).contains("analysis command exited unsuccessfully"));
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger(&sandbox)
            .by_id(&record.id)
            .unwrap()
            .unwrap()
            .stages
            .analysis,
        StageState::Failed
    );
}
