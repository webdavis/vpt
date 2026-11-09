use super::*;

#[test]
fn pipeline_recovery_and_new_moves_emit_one_combined_retention_event() {
    let (sandbox, _, artifacts) = prepared();
    let pending_artifact = artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::EngineOutput)
        .unwrap();
    pending(&sandbox, pending_artifact);
    synthesis::append(
        &sandbox,
        &format!(
            "\n[report]\ncommand=[{:?}, 'command-sink']\n",
            support::FAKE_ENGINE
        ),
    );
    age(artifacts.iter().map(|a| &a.path));
    let output = json(&invoke(&sandbox, &[]));
    assert_eq!(output["retention"]["moved"].as_array().unwrap().len(), 8);
    let log = fs::read_to_string(sandbox.fake_log()).unwrap();
    let events: Vec<serde_json::Value> = log
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|record| record["command"] == "command-sink")
        .map(|record| serde_json::from_str(record["stdin"].as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(
        events.len(),
        1,
        "one notification for all retention moves in this invocation"
    );
    assert_eq!(events[0]["counts"]["engine_outputs"], 1);
    assert_eq!(events[0]["counts"]["audio"], 1);
    assert_eq!(events[0]["counts"]["drafts"], 2);
    assert_eq!(events[0]["counts"]["released"], 2);
    assert!(ledger(&sandbox).pending_intents().unwrap().is_empty());
}

#[test]
fn retention_log_and_hook_report_actual_counts_for_each_store() {
    let (sandbox, _, artifacts) = prepared();
    synthesis::append(
        &sandbox,
        &format!(
            "\n[report]\ncommand=[{:?}, 'command-sink']\n",
            support::FAKE_ENGINE
        ),
    );
    age(artifacts.iter().map(|a| &a.path));
    json(&invoke(&sandbox, &[]));
    let records: Vec<serde_json::Value> = fs::read_to_string(sandbox.fake_log())
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let hooks: Vec<_> = records
        .iter()
        .filter(|r| r["command"] == "command-sink")
        .collect();
    assert_eq!(hooks.len(), 1);
    let event: serde_json::Value =
        serde_json::from_str(hooks[0]["stdin"].as_str().unwrap()).unwrap();
    assert_eq!(event["event"], "retention");
    for (store, count) in [
        ("audio", 1),
        ("engine_outputs", 1),
        ("transcripts", 1),
        ("analysis", 1),
        ("briefs", 0),
        ("drafts", 2),
        ("released", 2),
    ] {
        assert_eq!(event["counts"][store], count, "{store}");
    }
    let history: Vec<serde_json::Value> =
        fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
    let logged: Vec<_> = history
        .iter()
        .filter(|r| r["fact"]["event"]["event"] == "retention")
        .collect();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0]["fact"]["event"]["counts"], event["counts"]);
    assert!(!event.to_string().contains("Private words"));
    let before = fs::read(sandbox.fake_log()).unwrap();
    json(&invoke(&sandbox, &[]));
    assert_eq!(fs::read(sandbox.fake_log()).unwrap(), before);
}
