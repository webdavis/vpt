use super::*;

#[test]
fn later_synthesis_failure_keeps_final_findings_for_already_published_analysis() {
    let sandbox = configured();
    synthesis::enable(&sandbox);
    boundaries::alert_hook(&sandbox, 3);
    for index in 0..2 {
        add(&sandbox, &format!("{index}.m4a"), 1_787_604_456 + index);
    }
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[],"actions":[{"text":"private action","sources":[]}],"tags":[],"relations":[]}"#).unwrap();
    fs::write(sandbox.path().join("agent.sh"), "#!/bin/sh\n/bin/cat > \"$1.input\"\nprintf 'called\\n' >> \"$1.calls\"\ncount=$(/usr/bin/wc -l < \"$1.calls\")\nif [ \"$count\" -eq 2 ]; then exit 17; fi\n/bin/cat \"$1.proposal\"\n").unwrap();
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["completed"].as_array().unwrap().len(), 2);
    let alerts = boundaries::alerts(&sandbox);
    assert_eq!(alerts.len(), 2);
    assert_eq!(alerts[0]["counts"]["unsourced"], 1);
    assert!(alerts[1]["counts"]["unsourced"].is_null());
    assert_eq!(alerts[0]["counts"]["diagnostics"], 1);
    assert_eq!(alerts[1]["counts"]["diagnostics"], 1);
}

#[test]
fn synthesis_findings_join_the_final_review_alert_once_per_recording() {
    let sandbox = configured();
    synthesis::enable(&sandbox);
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[],"actions":[{"text":"private action","sources":[]}],"tags":[],"relations":[]}"#).unwrap();
    boundaries::alert_hook(&sandbox, 3);
    for index in 0..4 {
        add(&sandbox, &format!("{index}.m4a"), 1_787_604_456 + index);
    }
    json(&invoke(&sandbox, &[]));
    let alerts = boundaries::alerts(&sandbox);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["counts"]["recordings"], 4);
    assert_eq!(alerts[0]["counts"]["unsourced"], 4);
    assert_eq!(alerts[0]["counts"]["diagnostics"], 4);
    assert!(!alerts[0].to_string().contains("private action"));
    json(&invoke(&sandbox, &[]));
    assert_eq!(boundaries::alerts(&sandbox), alerts);
}

#[test]
fn standalone_synthesis_reports_findings_after_analysis_publication() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    synthesis::enable(&sandbox);
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[],"actions":[{"text":"private action","sources":[]}],"tags":[],"relations":[]}"#).unwrap();
    boundaries::alert_hook(&sandbox, 3);
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    json(&support::run_with_budget(
        sandbox
            .vpt()
            .args(["synthesize", row.id.as_str(), "--json"]),
        std::time::Duration::from_secs(5),
    ));
    let analysis = ledger(&sandbox)
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    let alerts = boundaries::alerts(&sandbox);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0]["counts"]["unsourced"], 1);
    assert_eq!(
        alerts[0]["paths"]["note"],
        analysis.target.to_str().unwrap()
    );
    assert_eq!(fs::read_to_string(&analysis.target).unwrap(), analysis.text);
}
