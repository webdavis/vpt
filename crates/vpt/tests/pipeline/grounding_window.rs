use super::*;
use vpt_domain::review::{FlagContent, Resolution};

#[test]
fn empty_checks_are_rechecked_for_new_windows_and_expired_analysis_stays_expired() {
    let sandbox = configured();
    analysis::enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    let base = config.split("[analysis]").next().unwrap();
    fs::write(
        sandbox.config_path(),
        format!("{base}\n[analysis]\ngrounding_window_secs=5\n"),
    )
    .unwrap();
    let snapshot = ledger(&sandbox).review_snapshot(&row.id).unwrap().unwrap();
    assert!(
        !snapshot
            .flags
            .iter()
            .any(|flag| matches!(flag.content, FlagContent::Verification(_)))
    );
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"][0]["stage"],
        "verify_analysis"
    );
    json(&invoke(&sandbox, &[]));
    let window = || {
        ledger(&sandbox)
            .review_snapshot(&row.id)
            .unwrap()
            .unwrap()
            .verified_claims
            .into_iter()
            .find(|claims| claims.artifact == NoteStage::Analysis)
            .unwrap()
            .grounding_window_secs
    };
    assert_eq!(window(), 5);
    assert!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let analysis = ledger(&sandbox)
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    fs::remove_file(&analysis.target).unwrap();
    sql(&sandbox, "UPDATE recordings SET stage_analysis='expired'");
    fs::write(
        sandbox.config_path(),
        format!("{base}\n[analysis]\ngrounding_window_secs=6\n"),
    )
    .unwrap();
    json(&invoke(&sandbox, &[]));
    assert!(!analysis.target.exists());
    assert_eq!(window(), 5);
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
}

#[test]
fn changed_grounding_window_rechecks_analysis_without_agent_and_keeps_review_decisions() {
    let sandbox = configured();
    analysis::enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    let engine = sandbox.path().join("engine.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&engine).unwrap()).unwrap();
    document["segments"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "start":10.0,"end":11.0,"text":"Acme", "words":[{"text":"Acme","start":10.0,"end":11.0}]
        }));
    fs::write(engine, serde_json::to_vec(&document).unwrap()).unwrap();
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Acme plan","sources":["00:00-00:01"]}],"actions":[{"text":"private task","sources":[]}],"tags":[],"relations":[]}"#).unwrap();
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let action = ledger(&sandbox).review_snapshot(&row.id).unwrap().unwrap().flags.into_iter()
        .find(|flag| matches!(&flag.content, FlagContent::Verification(finding) if finding.record_text == "private task")).unwrap();
    json(&support::run_with_budget(
        sandbox.vpt().args([
            "review",
            row.id.as_str(),
            "--resolve",
            &action.identifier.hex(),
            "--confirm",
            "--json",
        ]),
        std::time::Duration::from_secs(5),
    ));
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    let base = config.split("[analysis]").next().unwrap();
    for window in [1, 15] {
        fs::write(
            sandbox.config_path(),
            format!("{base}\n[analysis]\ngrounding_window_secs={window}\n"),
        )
        .unwrap();
        let before = fixture::snapshot(&sandbox);
        let plan = json(&invoke(&sandbox, &["--dry-run"]));
        assert_eq!(plan["plan"][0]["stage"], "verify_analysis");
        assert_eq!(fixture::snapshot(&sandbox), before);
        let result = json(&invoke(&sandbox, &[]));
        assert_eq!(result["analyzed"], serde_json::json!([]));
        let current = ledger(&sandbox).review_snapshot(&row.id).unwrap().unwrap();
        assert!(current.stale_artifacts.is_empty());
        assert!(matches!(
            current
                .flags
                .iter()
                .find(|flag| flag.identifier == action.identifier)
                .unwrap()
                .resolution,
            Resolution::Confirmed { .. }
        ));
        let analysis = ledger(&sandbox)
            .note(&row.id, NoteStage::Analysis)
            .unwrap()
            .unwrap();
        assert_eq!(analysis.text.contains("[unsupported: Acme]"), window == 1);
        assert!(!analysis.text.contains("[unsourced]"));
        let count = if window == 1 { 1 } else { 0 };
        for stage in [NoteStage::Transcript, NoteStage::Analysis] {
            assert!(
                ledger(&sandbox)
                    .note(&row.id, stage)
                    .unwrap()
                    .unwrap()
                    .text
                    .contains(&format!("vpt-open-flags: {count}"))
            );
        }
        assert!(
            json(&invoke(&sandbox, &["--dry-run"]))["plan"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
        assert_eq!(analysis::calls(&sandbox, "calls"), 1);
    }
}
