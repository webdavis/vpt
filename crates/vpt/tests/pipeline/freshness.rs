use super::*;

#[test]
fn changed_rendered_input_rechecks_transcript_before_analysis_and_then_skips_both() {
    let sandbox = configured();
    analysis::enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let transcript = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    json(&support::run_with_budget(
        sandbox.vpt().args([
            "verify-note",
            transcript.target.to_str().unwrap(),
            "--recording",
            row.id.as_str(),
            "--json",
        ]),
        std::time::Duration::from_secs(5),
    ));
    let old = ledger(&sandbox).review_snapshot(&row.id).unwrap().unwrap();
    assert!(
        old.verified_claims
            .iter()
            .any(|claims| claims.artifact == NoteStage::Transcript)
    );
    analysis::append(&sandbox, "\n[readability]\nenabled = true\n");
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"][0]["stage"],
        "note"
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    let result = json(&invoke(&sandbox, &[]));
    assert_eq!(result["analyzed"], serde_json::json!([row.id.as_str()]));
    let current = ledger(&sandbox).review_snapshot(&row.id).unwrap().unwrap();
    assert!(
        current.stale_artifacts.is_empty(),
        "stale checks must run before analysis"
    );
    assert_ne!(old.verified_claims, current.verified_claims);
    let transcript = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert!(transcript.text.contains("[unsourced]"));
    let analysis = ledger(&sandbox)
        .note(&row.id, NoteStage::Analysis)
        .unwrap()
        .unwrap();
    assert!(transcript.text.contains("vptOpenFlags: 1"));
    assert!(analysis.text.contains("vptOpenFlags: 1"));
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 2);
    let next = json(&invoke(&sandbox, &[]));
    assert_eq!(next["analyzed"], serde_json::json!([]));
    assert_eq!(next["notes"], serde_json::json!([]));
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 2);
    assert_eq!(analysis::calls(&sandbox, "calls"), 1);
}

#[test]
fn run_rechecks_stale_transcript_findings_with_analysis_disabled() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let transcript = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    json(&support::run_with_budget(
        sandbox.vpt().args([
            "verify-note",
            transcript.target.to_str().unwrap(),
            "--recording",
            row.id.as_str(),
            "--json",
        ]),
        std::time::Duration::from_secs(5),
    ));
    analysis::append(&sandbox, "\n[readability]\nenabled = true\n");
    json(&invoke(&sandbox, &[]));
    let current = ledger(&sandbox).review_snapshot(&row.id).unwrap().unwrap();
    assert!(current.stale_artifacts.is_empty());
    assert_eq!(current.verified_claims.len(), 1);
    assert!(!sandbox.path().join("agent.calls").exists());
    assert_eq!(analysis::calls(&sandbox, "calls"), 1);
}
