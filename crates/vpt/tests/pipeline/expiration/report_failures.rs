use super::*;

fn enable_report(sandbox: &Sandbox) {
    analysis::append(
        sandbox,
        &format!(
            "\n[report]\ncommand=[{:?}, 'command-sink']\n",
            support::FAKE_ENGINE
        ),
    );
}

fn events(sandbox: &Sandbox) -> Vec<serde_json::Value> {
    fs::read_to_string(sandbox.fake_log())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|record| record["command"] == "command-sink")
        .map(|record| serde_json::from_str(record["stdin"].as_str().unwrap()).unwrap())
        .collect()
}

#[test]
fn partial_recovery_refusal_reports_actual_moves_once_and_keeps_the_replacement() {
    let (sandbox, row, artifacts) = prepared();
    enable_report(&sandbox);
    pending(&sandbox, &artifacts[0]);
    pending(&sandbox, &artifacts[1]);
    fs::write(&artifacts[1].path, b"PRIVATE replacement").unwrap();
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["rule"], "retention_target_modified");
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    let events = events(&sandbox);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["state"], "failed");
    for store in vpt_domain::layout::StoreKey::all() {
        assert_eq!(
            events[0]["counts"][store.key_name()],
            u32::from(store == artifacts[0].kind.store())
        );
    }
    assert!(!artifacts[0].path.exists());
    assert_eq!(
        fs::read(&artifacts[1].path).unwrap(),
        b"PRIVATE replacement"
    );
    assert_eq!(ledger(&sandbox).pending_intents().unwrap().len(), 1);
    assert!(!events[0].to_string().contains("PRIVATE"));
    assert_eq!(analysis::calls(&sandbox, "calls"), 1);
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
}

#[test]
fn source_failure_after_recovery_still_reports_successful_retention_once() {
    let (sandbox, row, artifacts) = prepared();
    enable_report(&sandbox);
    let engine = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::EngineOutput)
        .unwrap();
    pending(&sandbox, engine);
    age(artifacts.iter().map(|a| &a.path));
    fs::remove_dir_all(sandbox.path().join("voice-memos/Recordings")).unwrap();
    let output = invoke(&sandbox, &[]);
    assert!(!output.status.success());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    let events = events(&sandbox);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["state"], "done");
    assert_eq!(events[0]["counts"]["engine_outputs"], 1);
    assert_eq!(events[0]["counts"]["audio"], 0);
    assert!(!engine.path.exists());
    assert!(
        artifacts
            .iter()
            .filter(|a| a.kind != ArtifactKind::EngineOutput)
            .all(|a| a.path.exists())
    );
    assert!(ledger(&sandbox).pending_intents().unwrap().is_empty());
}

#[test]
fn failed_summary_commit_keeps_durable_moves_and_does_not_send_or_replay_a_hook() {
    let (sandbox, row, artifacts) = prepared();
    enable_report(&sandbox);
    age(artifacts.iter().map(|a| &a.path));
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_retention_summary BEFORE INSERT ON history_records WHEN NEW.fact LIKE '%\"kind\":\"retention\"%' BEGIN SELECT RAISE(ABORT, 'PRIVATE summary failure'); END;",
    );
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert!(!stderr(&output).contains("PRIVATE"));
    assert!(artifacts.iter().all(|a| !a.path.exists()));
    assert!(ledger(&sandbox).pending_intents().unwrap().is_empty());
    assert!(events(&sandbox).is_empty());
    sql(&sandbox, "DROP TRIGGER refuse_retention_summary;");
    let before = fs::read(sandbox.fake_log()).unwrap();
    let retry = json(&invoke(&sandbox, &[]));
    assert_eq!(retry["retention"]["moved"], serde_json::json!([]));
    assert_eq!(fs::read(sandbox.fake_log()).unwrap(), before);
    assert!(artifacts.iter().all(|a| !a.path.exists()));
}
