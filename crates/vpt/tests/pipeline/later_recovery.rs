use super::*;

#[test]
fn pipeline_repairs_accepted_analysis_without_rerunning_agent_or_resending_hooks() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    analysis::enable(&sandbox);
    boundaries::alert_hook(&sandbox, 3);
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_analysis_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/analysis/%' BEGIN SELECT RAISE(ABORT, 'Private ack'); END",
    );
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let accepted = ledger(&sandbox)
        .accepted_proposal(&row.id)
        .unwrap()
        .unwrap();
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert!(boundaries::alerts(&sandbox).is_empty());
    sql(&sandbox, "DROP TRIGGER refuse_analysis_ack");
    let before = fixture::snapshot(&sandbox);
    json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(fixture::snapshot(&sandbox), before);
    json(&invoke(&sandbox, &[]));
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
    assert_eq!(
        ledger(&sandbox).accepted_proposal(&row.id).unwrap(),
        Some(accepted)
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert!(boundaries::alerts(&sandbox).is_empty());
}

#[test]
fn stale_analysis_is_rechecked_with_disabled_analysis_and_an_expired_transcript() {
    let sandbox = configured();
    analysis::enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(
        sandbox.config_path(),
        config.split("[analysis]").next().unwrap(),
    )
    .unwrap();
    let engine = sandbox.path().join("engine.json");
    fs::write(
        &engine,
        fs::read_to_string(&engine)
            .unwrap()
            .replace("Private", "Different"),
    )
    .unwrap();
    json(&support::run_with_budget(
        sandbox
            .vpt()
            .args(["transcribe", row.id.as_str(), "--json"]),
        std::time::Duration::from_secs(5),
    ));
    let transcript = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    fs::remove_file(&transcript.target).unwrap();
    sql(&sandbox, "UPDATE recordings SET stage_note='expired'");
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"][0]["stage"],
        "note"
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    boundaries::alert_hook(&sandbox, 3);
    json(&invoke(&sandbox, &[]));
    let current = ledger(&sandbox).note_inputs(&row.id).unwrap().unwrap();
    assert!(
        !current
            .review
            .stale_artifacts
            .contains(&NoteStage::Analysis)
    );
    assert!(!transcript.target.exists());
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
    let analysis = ledger(&sandbox)
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    assert!(analysis.text.contains("[unsupported: Private]"));
    let alerts = boundaries::alerts(&sandbox);
    assert_eq!(alerts.len(), 1);
    assert_eq!(
        alerts[0]["paths"]["note"],
        analysis.target.to_str().unwrap()
    );
}
