use super::*;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::RunHistory;
use vpt_domain::run::HistoryFact;

fn history(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

fn documents(path: &std::path::Path) -> Vec<serde_json::Value> {
    let bytes = fs::read(path).unwrap();
    assert!(bytes.ends_with(b"\n"));
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

#[test]
fn failed_export_and_hook_recover_persisted_history_without_repeating_notifications() {
    let sandbox = configured();
    let source = add(&sandbox, "partial.m4a");
    fs::write(source, b"private audio canary").unwrap();
    fixture::seed(&sandbox, vec![]);
    let log = sandbox.path().join("state/vpt/events.jsonl");
    fs::create_dir(&log).unwrap();
    let hook = sandbox.path().join("bin/alert");
    let captured = sandbox.path().join("alerts.jsonl");
    fs::write(&hook, "#!/bin/sh\n/bin/cat >> \"$1\"\nprintf '\\n' >> \"$1\"\nprintf 'private hook stdout'\nprintf 'private hook stderr' >&2\nexit 7\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}deferral_page_threshold = 1\n[alert]\ncommand = [{hook:?}, {captured:?}]\n[report]\ncommand = ['/usr/bin/touch', {:?}]\n", sandbox.path().join("report-called"))).unwrap();

    let first = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    let diagnostics = first["diagnostics"].as_array().unwrap();
    assert!(diagnostics.contains(&serde_json::json!("event log export is unavailable")));
    assert!(diagnostics.contains(&serde_json::json!("configured notification command failed")));
    assert_eq!(first["deferred"].as_array().unwrap().len(), 1);
    for private in [
        "private audio canary",
        "private hook stdout",
        "private hook stderr",
    ] {
        assert!(!first.to_string().contains(private));
    }
    let delivered = fs::read(&captured).unwrap();
    assert_eq!(documents(&captured).len(), 1);
    assert!(!sandbox.path().join("report-called").exists());
    let before = history(&sandbox).recovery_view().unwrap();
    assert!(!before.failures.is_empty());
    assert!(
        before
            .records
            .iter()
            .all(|record| record.export_position.is_none())
    );
    assert_eq!(
        before
            .records
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::HookFailed { .. }))
            .count(),
        1
    );

    let snapshot_before = snapshot(&sandbox);
    json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
    assert_eq!(snapshot(&sandbox), snapshot_before);
    fs::rename(&log, sandbox.path().join("held-log-directory")).unwrap();
    let first_record = &before.records[0];
    let prefix = vpt_protocol::encode_history(first_record, first_record.occurred_at).unwrap();
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&log)
        .unwrap();
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .unwrap();
    file.write_all(prefix.as_bytes()).unwrap();
    file.write_all(b"\n{incomplete-tail").unwrap();
    file.sync_all().unwrap();
    drop(file);

    json(&run(sandbox.vpt().args(["ingest", "--json"])));
    assert_eq!(fs::read(&captured).unwrap(), delivered);
    assert!(!sandbox.path().join("report-called").exists());
    let restored = documents(&log);
    assert_eq!(restored[0]["event_id"], restored[1]["event_id"]);
    assert_eq!(restored[0]["event_id"], first_record.id.render());
    assert_eq!(restored[0]["occurred_at"], restored[1]["occurred_at"]);
    assert_eq!(restored[0]["fact"], restored[1]["fact"]);
    let saved = history(&sandbox).recovery_view().unwrap();
    for old in &before.records {
        let new = saved
            .records
            .iter()
            .find(|record| record.id == old.id)
            .unwrap();
        assert_eq!(
            (new.run, new.occurred_at, &new.fact),
            (old.run, old.occurred_at, &old.fact)
        );
        assert!(new.export_position.is_some());
        assert!(new.appended_at.is_some());
    }
    let summaries: Vec<_> = saved
        .records
        .iter()
        .filter_map(|record| match &record.fact {
            HistoryFact::RecoverySummary { evidence, .. } => Some(evidence),
            _ => None,
        })
        .collect();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].affected_runs, vec![first_record.run]);
    assert_eq!(
        summaries[0].recovered_records as usize,
        before.records.len()
    );
    assert_eq!(summaries[0].deferred_work, 1);
    assert_eq!(summaries[0].completed_work, 0);
    assert_eq!(
        summaries[0].first_failure_observed_at,
        Some(before.failures[0].failure.observed_at)
    );
    assert!(
        summaries[0].logging_restored_observed_at.unwrap().secs
            >= before.failures[0].failure.observed_at.secs
    );
    assert!(history(&sandbox).pending_exports().unwrap().is_empty());
    assert_eq!(
        fs::metadata(&log).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let bytes = fs::read_to_string(&log).unwrap();
    for private in [
        "private audio canary",
        "private hook stdout",
        "private hook stderr",
        "incomplete-tail",
    ] {
        assert!(!bytes.contains(private));
    }

    json(&run(sandbox.vpt().args(["ingest", "--json"])));
    assert_eq!(fs::read(captured).unwrap(), delivered);
    assert_eq!(
        history(&sandbox)
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::RecoverySummary { .. }))
            .count(),
        1
    );
}
