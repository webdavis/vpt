use super::*;

fn enable(sandbox: &Sandbox) {
    analysis::append(sandbox, "\n[share]\nautomatic = true\n");
}

#[test]
fn automatic_sharing_without_analysis_releases_once_and_dry_run_is_observational() {
    let sandbox = configured();
    enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["redacted"],
        serde_json::json!([])
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    let first = json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let copies = first["redacted"].as_array().unwrap();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0]["id"], row.id.as_str());
    assert_eq!(copies[0]["stage"], "transcript");
    assert!(std::path::Path::new(copies[0]["written"].as_str().unwrap()).is_file());
    assert!(std::path::Path::new(copies[0]["report"].as_str().unwrap()).is_file());
    let sequences = ledger(&sandbox).releases(&row.id).unwrap();
    assert_eq!(sequences.len(), 1);
    assert_eq!(
        json(&invoke(&sandbox, &[]))["redacted"],
        serde_json::json!([])
    );
    assert_eq!(ledger(&sandbox).releases(&row.id).unwrap(), sequences);
    assert_eq!(analysis::calls(&sandbox, "calls"), 1);
}

#[test]
fn automatic_sharing_releases_each_source_stage_and_reports_candidates_in_human_output() {
    let sandbox = configured();
    analysis::enable(&sandbox);
    enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Private words with Candidate","sources":["00:00-00:01"]}],"actions":[],"tags":[],"relations":[]}"#).unwrap();
    let first = json(&invoke(&sandbox, &[]));
    let copies = first["redacted"].as_array().unwrap();
    assert_eq!(copies.len(), 2);
    assert_eq!(copies[0]["stage"], "transcript");
    assert_eq!(copies[1]["stage"], "analysis");
    assert!(
        copies[1]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["token"] == "Candidate")
    );
    assert_eq!(
        json(&invoke(&sandbox, &[]))["redacted"],
        serde_json::json!([])
    );
    assert_eq!(analysis::calls(&sandbox, "agent.calls"), 1);
}

#[test]
fn transcript_release_survives_later_analysis_failure_and_retry_adds_only_analysis() {
    let sandbox = configured();
    analysis::enable(&sandbox);
    enable(&sandbox);
    add(&sandbox, "one.m4a", 1_787_604_456);
    let path = sandbox.path().join("agent.proposal");
    let valid = fs::read(&path).unwrap();
    fs::write(&path, "malformed proposal").unwrap();
    let failed = invoke(&sandbox, &[]);
    assert_eq!(failed.status.code(), Some(1));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let sequences = ledger(&sandbox).releases(&row.id).unwrap();
    assert_eq!(sequences.len(), 1);
    assert_eq!(
        ledger(&sandbox)
            .release(sequences[0])
            .unwrap()
            .unwrap()
            .stage,
        NoteStage::Transcript
    );
    let error: serde_json::Value = serde_json::from_str(&stderr(&failed)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    fs::write(path, valid).unwrap();
    let retried = json(&invoke(&sandbox, &[]));
    assert_eq!(retried["redacted"].as_array().unwrap().len(), 1);
    assert_eq!(retried["redacted"][0]["stage"], "analysis");
    assert_eq!(ledger(&sandbox).releases(&row.id).unwrap().len(), 2);
}

#[test]
fn sharing_is_off_by_default_and_enabling_it_processes_existing_notes() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    assert_eq!(
        json(&invoke(&sandbox, &[]))["redacted"],
        serde_json::json!([])
    );
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    assert!(ledger(&sandbox).releases(&row.id).unwrap().is_empty());
    enable(&sandbox);
    assert_eq!(
        json(&invoke(&sandbox, &[]))["redacted"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn sharing_plan_is_observational_and_human_run_lists_surviving_candidates() {
    let sandbox = configured();
    analysis::enable(&sandbox);
    fs::write(sandbox.path().join("agent.proposal"), r#"{"schema":"vpt.proposal/1","by":"fixture","summary":[{"text":"Private words with Candidate","sources":["00:00-00:01"]}],"actions":[],"tags":[],"relations":[]}"#).unwrap();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    enable(&sandbox);
    let before = fixture::snapshot(&sandbox);
    let plan = json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(plan["plan"][0]["stage"], "redact_transcript");
    assert_eq!(fixture::snapshot(&sandbox), before);
    let output =
        support::run_with_budget(sandbox.vpt().arg("run"), std::time::Duration::from_secs(8));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("2 released"));
    assert!(stdout(&output).contains("Candidate, line"));
    assert!(stdout(&output).contains("Candidate"));
    assert_eq!(
        json(&invoke(&sandbox, &["--dry-run"]))["plan"],
        serde_json::json!([])
    );
}

#[test]
fn pending_automatic_release_recovers_before_unchanged_work_without_an_extra_sequence() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    enable(&sandbox);
    sql(
        &sandbox,
        "CREATE TRIGGER fail_release BEFORE INSERT ON release_completions BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
    );
    let failed = invoke(&sandbox, &[]);
    assert_eq!(failed.status.code(), Some(1));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let pending = ledger(&sandbox).pending_releases().unwrap();
    assert_eq!(pending.len(), 1);
    let payload = ledger(&sandbox)
        .release_payload(pending[0])
        .unwrap()
        .unwrap();
    let before = fs::read(&payload.release.destination).unwrap();
    sql(&sandbox, "DROP TRIGGER fail_release;");
    let recovered = json(&invoke(&sandbox, &[]));
    assert_eq!(recovered["redacted"], serde_json::json!([]));
    assert_eq!(ledger(&sandbox).releases(&row.id).unwrap(), pending);
    assert!(ledger(&sandbox).pending_releases().unwrap().is_empty());
    assert_eq!(fs::read(&payload.release.destination).unwrap(), before);
}
