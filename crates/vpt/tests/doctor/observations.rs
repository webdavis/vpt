use crate::fixture::{check, configured, detail, issues, run_json, seed_seen, snapshot};
use vpt_adapters::{RootDir, WriteLock};
use vpt_application::ports::StageStates;

#[test]
fn source_gone_counts_only_marked_rows() {
    let sandbox = configured("doctor-source-gone");
    seed_seen(&sandbox, true);
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "ledger", "source_gone"),
        "1 recordings whose source is gone"
    );
    assert_eq!(
        detail(&document, "ledger", "schema")
            .parse::<u32>()
            .map(|_| ()),
        Ok(())
    );
    seed_seen(&sandbox, false);
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "ledger", "source_gone"),
        "0 recordings whose source is gone"
    );
}

#[test]
fn doctor_preserves_populated_dirty_state() {
    let sandbox = configured("doctor-dirty");
    let row = crate::observations_fixture::record(&sandbox, 1, StageStates::fresh());
    crate::observations_fixture::dirty(&sandbox, row);
    let before = snapshot(&sandbox);
    run_json(&sandbox, 0);
    assert_eq!(snapshot(&sandbox), before);
}

fn missing(companion: &str) {
    let sandbox = configured("doctor-missing-companion");
    seed_seen(&sandbox, false);
    drop(crate::observations_fixture::uncheckpointed(
        &sandbox,
        "UPDATE seen SET size = 2",
    ));
    let state = sandbox.path().join("state/vpt");
    std::fs::rename(state.join(companion), sandbox.path().join("held-companion")).unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    assert_eq!(check(&document, "ledger")["result"], "issue");
    assert_eq!(failing_details(&document), ["ledger operation failed"]);
    assert_eq!(snapshot(&sandbox), before);
}

fn failing_details(document: &serde_json::Value) -> Vec<String> {
    assert!(
        issues(document)
            .iter()
            .all(|issue| issue.starts_with("ledger ")),
        "{document}"
    );
    document["needs_attention"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["result"] == "issue")
        .map(|entry| entry["detail"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn missing_wal_fails_only_the_ledger() {
    missing("vpt.db-wal");
}
#[test]
fn missing_shm_fails_only_the_ledger() {
    missing("vpt.db-shm");
}

fn linked(leaf: &str) {
    let sandbox = configured("doctor-linked-ledger");
    seed_seen(&sandbox, false);
    let path = sandbox.path().join("state/vpt").join(leaf);
    let held = sandbox.path().join("held-leaf");
    std::fs::rename(&path, &held).unwrap();
    std::os::unix::fs::symlink(&held, &path).unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    let failing = failing_details(&document);
    assert_eq!(failing.len(), 1, "{document}");
    assert!(failing[0].contains("escapes"), "{document}");
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn linked_main_remains_aggregate_and_unchanged() {
    linked("vpt.db");
}
#[test]
fn linked_wal_remains_aggregate_and_unchanged() {
    linked("vpt.db-wal");
}
#[test]
fn linked_shm_remains_aggregate_and_unchanged() {
    linked("vpt.db-shm");
}

#[test]
fn held_application_lock_does_not_block_doctor() {
    let sandbox = configured("doctor-write-lock");
    seed_seen(&sandbox, false);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let _held = WriteLock::acquire(&root, std::time::Duration::ZERO).unwrap();
    let before = snapshot(&sandbox);
    run_json(&sandbox, 0);
    assert_eq!(snapshot(&sandbox), before);
}
