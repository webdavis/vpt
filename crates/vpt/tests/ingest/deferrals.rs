use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::RecordingLedger;

fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

fn alerts(sandbox: &Sandbox) -> Vec<serde_json::Value> {
    fs::read_to_string(sandbox.path().join("alerts.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn persistent_deferrals_alert_once_at_default_and_custom_threshold_then_recover() {
    for threshold in [4, 2] {
        let sandbox = configured();
        let path = add(&sandbox, "partial.m4a");
        fs::write(&path, b"private incomplete audio canary").unwrap();
        let script = sandbox.path().join("bin/alert");
        fs::write(
            &script,
            "#!/bin/sh\n/bin/cat >> \"$1\"\nprintf '\\n' >> \"$1\"\n",
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let mut config = fs::read_to_string(sandbox.config_path()).unwrap();
        if threshold != 4 {
            config.push_str(&format!("deferral_page_threshold = {threshold}\n"));
        }
        config.push_str(&format!(
            "[alert]\ncommand = [{script:?}, {:?}]\n[report]\ncommand = ['/usr/bin/touch', {:?}]\n",
            sandbox.path().join("alerts.jsonl"),
            sandbox.path().join("report-called")
        ));
        fs::write(sandbox.config_path(), config).unwrap();
        let mut first_seen = None;
        for count in 1..=threshold + 1 {
            let result = json(&run(sandbox.vpt().args(["ingest", "--json"])));
            assert_eq!(
                result["deferred"],
                serde_json::json!([{"path":path,"reason":"invalid_container"}])
            );
            assert_eq!(result["ingested"], serde_json::json!([]));
            let seen = ledger(&sandbox).seen(&path).unwrap().unwrap();
            assert_eq!(seen.deferral_count, count);
            assert_eq!(*first_seen.get_or_insert(seen.first_seen), seen.first_seen);
            assert_eq!(alerts(&sandbox).len(), usize::from(count >= threshold));
        }
        let event = alerts(&sandbox).remove(0);
        assert_eq!(event["schema"], "vpt.event/1");
        assert_eq!(event["event"], "deferred");
        assert_eq!(event["channel"], "alert");
        assert_eq!(
            event["detail"],
            format!("deferred {threshold} times: invalid_container")
        );
        assert_eq!(event["paths"]["source"], path.to_str().unwrap());
        assert!(
            !event
                .to_string()
                .contains("private incomplete audio canary")
        );
        assert!(!sandbox.path().join("report-called").exists());
        let history = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
        assert!(!history.contains("private incomplete audio canary"));
        assert_eq!(
            history
                .lines()
                .filter(|line| line.contains("\"event\":\"deferred\""))
                .count(),
            1
        );
        let before = snapshot(&sandbox);
        json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
        assert_eq!(snapshot(&sandbox), before);
        fs::write(&path, m4a(1_787_604_456, 12, b"repaired audio")).unwrap();
        fs::File::open(&path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH)
            .unwrap();
        let settling = json(&run(sandbox.vpt().args(["ingest", "--json"])));
        assert_eq!(settling["deferred"][0]["reason"], "not_at_rest");
        let accepted = json(&run(sandbox.vpt().args(["ingest", "--json"])));
        assert_eq!(accepted["ingested"].as_array().unwrap().len(), 1);
        let seen = ledger(&sandbox).seen(&path).unwrap().unwrap();
        assert_eq!(seen.deferral_count, 0);
        assert!(seen.deferral_reason.is_none());
        assert!(seen.deferred_size.is_none());
        assert!(seen.recording.is_some());
        assert_eq!(alerts(&sandbox).len(), 1);
    }
}
