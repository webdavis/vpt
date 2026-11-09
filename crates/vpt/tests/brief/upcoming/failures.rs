use super::*;

#[test]
fn completion_and_report_rollback_together_and_partial_success_keeps_its_identity() {
    let sandbox = configured();
    fixture::seed(&sandbox, vec![]);
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    connection.execute_batch("CREATE TRIGGER refuse_second_brief BEFORE DELETE ON dirty_publications WHEN (SELECT count(*) FROM occasions WHERE briefed_at IS NOT NULL) >= 2 BEGIN SELECT RAISE(ABORT,'Private interruption'); END").unwrap();
    fixture::preserve_wal(&connection);
    let now = SystemClock.now().secs;
    reply(
        &sandbox,
        vec![event("one", now + 3600), event("two", now + 7200)],
    );
    let output = invoke(&sandbox, &["brief", "--upcoming", "--json"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private"));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let rows = ledger.occasions().unwrap();
    let done: Vec<_> = rows.iter().filter(|row| row.briefed_at.is_some()).collect();
    assert_eq!(done.len(), 1);
    assert_eq!(error["error"]["completed"], json!([done[0].id.as_str()]));
    assert_eq!(reports(&sandbox).len(), 1);
    assert_eq!(ledger.pending_publications().unwrap().len(), 1);
    assert!(rows.iter().any(|row| row.briefed_at.is_none()));
    connection
        .execute_batch("DROP TRIGGER refuse_second_brief")
        .unwrap();
    drop(connection);
    assert_eq!(
        json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]))["written"],
        json!([])
    );
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert!(
        ledger
            .occasions()
            .unwrap()
            .iter()
            .all(|row| row.briefed_at.is_some())
    );
    assert_eq!(
        reports(&sandbox).len(),
        1,
        "startup recovery must not replay notifications"
    );
}

#[test]
fn broken_stdout_keeps_the_completed_upcoming_occasion_and_does_not_repeat_its_report() {
    let sandbox = configured();
    reply(&sandbox, vec![event("one", SystemClock.now().secs + 3600)]);
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args(["brief", "--upcoming", "--json"]),
        std::time::Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let reports = reports(&sandbox);
    assert_eq!(reports.len(), 1);
    assert_eq!(
        error["error"]["completed"],
        json!([reports[0]["fact"]["event"]["occasion"]])
    );
    assert_eq!(
        json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]))["written"],
        json!([])
    );
    assert_eq!(super::reports(&sandbox).len(), 1);
}
