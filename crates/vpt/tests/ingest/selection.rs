use super::*;
use std::fs;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::RecordingLedger;

#[test]
fn once_and_sweep_select_only_immediate_recordings_without_changing_sources() {
    let sandbox = configured();
    let first = add(&sandbox, "first.m4a");
    let second = add(&sandbox, "second.m4a");
    fs::write(&second, m4a(1_787_604_457, 7, b"second private recording")).unwrap();
    fs::File::open(&second)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH)
        .unwrap();
    let source = first.parent().unwrap();
    for name in ["sidecar.plist", "recording.M4A", "CloudRecordings.db"] {
        fs::write(source.join(name), b"ignored private bytes").unwrap();
    }
    for name in vpt_adapters::APPLE_SUBDIRECTORIES
        .into_iter()
        .chain(["directory.m4a"])
    {
        fs::create_dir(source.join(name)).unwrap();
        fs::write(
            source.join(name).join("nested.m4a"),
            m4a(1_787_604_458, 9, b"nested"),
        )
        .unwrap();
    }
    std::os::unix::fs::symlink(&first, source.join("linked.m4a")).unwrap();
    let before = snapshot(&sandbox);
    let dry = json(&run(sandbox
        .vpt()
        .args(["ingest", "--once"])
        .arg(&second)
        .args(["--dry-run", "--json"])));
    assert_eq!(dry["would_ingest"].as_array().unwrap().len(), 1);
    assert_eq!(dry["would_ingest"][0]["path"], second.to_str().unwrap());
    assert_eq!(snapshot(&sandbox), before);
    let once = json(&run(sandbox
        .vpt()
        .args(["ingest", "--once"])
        .arg(&second)
        .arg("--json")));
    assert_eq!(once["ingested"].as_array().unwrap().len(), 1);
    let sweep = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    assert_eq!(sweep["ingested"].as_array().unwrap().len(), 1);
    assert_eq!(
        sweep["already_ingested"],
        serde_json::json!([once["ingested"][0]["id"]])
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let rows = ledger.seen_all().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|row| row.path == first || row.path == second)
    );
    // Compare the entire source tree, including modes, flags, times, and sidecar bytes.
    assert_eq!(
        before
            .into_iter()
            .filter(|entry| entry.is_within(source))
            .collect::<Vec<_>>(),
        snapshot(&sandbox)
            .into_iter()
            .filter(|entry| entry.is_within(source))
            .collect::<Vec<_>>()
    );
}

#[test]
fn once_preserves_source_gates_in_observation_and_mutating_modes() {
    for reason in ["audio_too_large", "invalid_container", "not_at_rest"] {
        let sandbox = configured();
        let path = add(&sandbox, "candidate.m4a");
        let config = fs::read_to_string(sandbox.config_path()).unwrap();
        match reason {
            "audio_too_large" => fs::write(
                sandbox.config_path(),
                format!("{config}max_audio_bytes = 1\n"),
            )
            .unwrap(),
            "invalid_container" => fs::write(&path, b"private malformed container").unwrap(),
            _ => fs::File::open(&path)
                .unwrap()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(60))
                .unwrap(),
        }
        let before = snapshot(&sandbox);
        let dry = json(&run(sandbox
            .vpt()
            .args(["ingest", "--once"])
            .arg(&path)
            .args(["--dry-run", "--json"])));
        assert_eq!(
            dry["deferred"],
            serde_json::json!([{"path": path, "reason": reason}])
        );
        assert_eq!(snapshot(&sandbox), before);
        let actual = json(&run(sandbox
            .vpt()
            .args(["ingest", "--once"])
            .arg(&path)
            .arg("--json")));
        assert_eq!(actual["deferred"], dry["deferred"]);
        assert_eq!(actual["ingested"], serde_json::json!([]));
        let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap();
        assert!(ledger.recordings().unwrap().is_empty());
        let seen = ledger.seen(&path).unwrap().unwrap();
        assert_eq!(seen.deferral_count, 1);
        assert_eq!(seen.deferral_reason.unwrap().as_str(), reason);
    }
}

#[test]
fn once_refuses_linked_nonregular_and_nested_paths_without_observational_writes() {
    let sandbox = configured();
    let original = add(&sandbox, "original.m4a");
    let source = original.parent().unwrap();
    let linked = source.join("linked.m4a");
    std::os::unix::fs::symlink(&original, &linked).unwrap();
    let directory = source.join("directory.m4a");
    fs::create_dir(&directory).unwrap();
    let nested = directory.join("nested.m4a");
    fs::copy(&original, &nested).unwrap();
    for (path, code, kind) in [
        (linked, 3, "refused"),
        (directory, 2, "usage"),
        (nested, 3, "refused"),
    ] {
        let before = snapshot(&sandbox);
        let result = run(sandbox
            .vpt()
            .args(["ingest", "--once"])
            .arg(path)
            .args(["--dry-run", "--json"]));
        assert_eq!(result.status.code(), Some(code));
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["schema"], "vpt.error/1");
        assert_eq!(error["error"]["kind"], kind);
        assert_eq!(snapshot(&sandbox), before);
    }
}
