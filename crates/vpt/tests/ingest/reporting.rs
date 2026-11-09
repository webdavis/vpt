use super::*;

#[test]
fn unreadable_source_records_one_safe_failure_event() {
    let sandbox = configured();
    std::fs::remove_dir(sandbox.path().join("voice-memos/Recordings")).unwrap();
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let log = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert_eq!(
        log.lines()
            .filter(|line| line.contains("ingest_failed"))
            .count(),
        1
    );
}

#[test]
fn unavailable_log_export_warns_and_preserves_ingestion_result() {
    let sandbox = configured();
    fixture::seed(&sandbox, vec![]);
    std::fs::create_dir(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    add(&sandbox, "a.m4a");
    let doc = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    assert_eq!(doc["ingested"].as_array().unwrap().len(), 1);
    assert!(
        doc["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("export"))
    );
}

#[test]
fn dry_run_ignores_write_lock_and_preserves_existing_state_and_title_copy() {
    let sandbox = configured();
    add(&sandbox, "a.m4a");
    json(&run(sandbox.vpt().args(["ingest", "--json"])));
    let root = vpt_adapters::RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let _lock = vpt_adapters::WriteLock::acquire(&root, std::time::Duration::ZERO).unwrap();
    let before = snapshot(&sandbox);
    let doc = json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
    assert_eq!(doc["skipped"], 1);
    assert_eq!(snapshot(&sandbox), before);
}
