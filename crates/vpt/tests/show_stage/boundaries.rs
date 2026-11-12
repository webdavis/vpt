use super::*;

fn sql(sandbox: &Sandbox, statement: &str) {
    let c = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&c);
    c.execute_batch(statement).unwrap();
}

#[test]
fn changed_non_utf8_release_is_an_artifact_refusal() {
    let (sandbox, id) = configured("show-invalid-text");
    let released = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    fs::write(released["written"].as_str().unwrap(), [255, 254]).unwrap();
    let output = show(&sandbox, id.as_str(), "released");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["error"]["rule"],
        "artifact_changed"
    );
}

#[test]
fn pending_release_is_an_operational_failure_without_recovery_or_publication() {
    let (sandbox, id) = configured("show-pending");
    sql(
        &sandbox,
        "CREATE TRIGGER fail_release BEFORE INSERT ON release_completions BEGIN SELECT RAISE(ABORT, 'fixture'); END;",
    );
    assert_eq!(
        invoke(&sandbox, &["redact", id.as_str(), "--json"])
            .status
            .code(),
        Some(1)
    );
    let before = fixture::snapshot(&sandbox);
    let output = show(&sandbox, id.as_str(), "released");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(ledger(&sandbox).pending_releases().unwrap().len(), 1);
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn brief_resolves_occasion_sources_and_keeps_operator_prose() {
    let (sandbox, id) = configured("show-brief");
    json(&invoke(
        &sandbox,
        &["confirm", id.as_str(), "--tag", "planning", "--json"],
    ));
    let result = json(&invoke(
        &sandbox,
        &[
            "brief",
            "--title",
            "Planning",
            "--at",
            "2026-08-25T10:00:00-06:00",
            "--tag",
            "planning",
            "--json",
        ],
    ));
    let occasion = result["occasion"]["id"].as_str().unwrap();
    let key = vpt_domain::occasion::OccasionId::parse(occasion).unwrap();
    let saved = ledger(&sandbox).brief(&key).unwrap().unwrap();
    assert_eq!(saved.snapshot.pack.sources.len(), 1);
    fs::write(
        &saved.target,
        format!("{}\nOperator brief text.\n", saved.text),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let brief = json(&show(&sandbox, occasion, "brief"));
    super::text::text_fields(&brief, "brief");
    assert_eq!(brief["stage"], "brief");
    assert_eq!(brief["occasion"], occasion);
    assert_eq!(brief["recordings"], json!([id.as_str()]));
    assert!(brief.get("recording").is_none());
    assert_eq!(
        brief["provenance"]["occasion_at"],
        "2026-08-25T10:00:00-06:00"
    );
    assert_eq!(brief["provenance"]["engines"], json!(["apple:fixture"]));
    let content = brief["content"].as_str().unwrap();
    assert!(content.ends_with("Operator brief text.\n"));
    assert!(!content.contains("vptOccasion:"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn human_output_is_the_text_and_broken_stdout_claims_no_completed_work() {
    let (sandbox, id) = configured("show-output");
    let expected = json(&show(&sandbox, id.as_str(), "transcript"));
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &["show", id.as_str(), "--stage", "transcript"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        support::stdout(&output),
        expected["content"].as_str().unwrap()
    );
    let failed = support::run_with_broken_stdout(
        sandbox
            .vpt()
            .args(["show", id.as_str(), "--stage", "transcript", "--json"]),
        std::time::Duration::from_secs(5),
    );
    assert_eq!(failed.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&failed.stderr).unwrap()["error"]["completed"],
        json!([])
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}
