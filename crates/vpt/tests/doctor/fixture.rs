use crate::support::{Sandbox, run, stderr, stdout};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Output;
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::{RecordingLedger, SeenRow};
use vpt_domain::time::{FileTime, UtcInstant};

pub(super) use crate::observations_fixture::snapshot;

pub(super) const CHECKS: [&str; 11] = [
    "config",
    "stores",
    "ledger",
    "recordings",
    "helper",
    "engines",
    "speakers",
    "summary",
    "actions",
    "calendar",
    "leftovers",
];

pub(super) fn configured(name: &str) -> Sandbox {
    let sandbox = crate::observations_fixture::configured(name);
    std::fs::create_dir(sandbox.path().join(".git")).unwrap();
    sandbox.install_fake_helper();
    configure(&sandbox, "");
    sandbox
}

pub(super) fn configure(sandbox: &Sandbox, extra: &str) {
    crate::observations_fixture::write_config(
        sandbox,
        &format!(
            "[source]\nrecordings_dir = {:?}\n{extra}",
            sandbox.path().join("voice-memos/Recordings")
        ),
    );
}

pub(super) fn run_json(sandbox: &Sandbox, code: i32) -> Value {
    report(
        &run(sandbox
            .vpt()
            .current_dir(sandbox.path())
            .args(["doctor", "-o", "json"])),
        code,
    )
}

pub(super) fn report(output: &Output, code: i32) -> Value {
    assert_eq!(output.status.code(), Some(code), "{}", stderr(output));
    assert!(output.stderr.is_empty(), "{}", stderr(output));
    let document: Value = serde_json::from_str(&stdout(output)).unwrap();
    assert_eq!(document["schema"], "vpt.result/1");
    assert_eq!(document["command"], "doctor");
    assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
    let checks = document["checks"].as_array().unwrap();
    let mut attention = (Vec::new(), Vec::new());
    for check in checks {
        let rows = check["rows"].as_array().unwrap();
        let results: Vec<&str> = rows
            .iter()
            .map(|row| row["result"].as_str().unwrap())
            .collect();
        let expected = if results.contains(&"issue") {
            "issue"
        } else if results.contains(&"warning") {
            "warning"
        } else if results.iter().all(|result| *result == "not_run") {
            "not_run"
        } else {
            "ok"
        };
        assert_eq!(check["result"], expected, "{check}");
        for row in rows {
            let entry = json!({"check": check["name"], "name": row["name"], "result": row["result"],
                "detail": row["detail"], "advice": row["advice"]});
            match row["result"].as_str().unwrap() {
                "issue" => attention.0.push(entry),
                "warning" => attention.1.push(entry),
                "ok" | "not_run" => {}
                other => panic!("unknown result {other}"),
            }
        }
    }
    assert_eq!(code == 1, !attention.0.is_empty(), "{document}");
    attention.0.extend(attention.1);
    assert_eq!(document["needs_attention"], Value::Array(attention.0));
    document
}

pub(super) fn names(document: &Value) -> Vec<&str> {
    document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["name"].as_str().unwrap())
        .collect()
}

pub(super) fn check<'a>(document: &'a Value, name: &str) -> &'a Value {
    document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == name)
        .unwrap_or_else(|| panic!("no check {name}: {document}"))
}

pub(super) fn row<'a>(document: &'a Value, check_name: &str, name: &str) -> &'a Value {
    check(document, check_name)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no row {check_name} {name}: {document}"))
}

pub(super) fn result<'a>(document: &'a Value, check_name: &str, name: &str) -> &'a str {
    row(document, check_name, name)["result"].as_str().unwrap()
}

pub(super) fn detail<'a>(document: &'a Value, check_name: &str, name: &str) -> &'a str {
    row(document, check_name, name)["detail"].as_str().unwrap()
}

pub(super) fn advice<'a>(document: &'a Value, check_name: &str, name: &str) -> &'a str {
    row(document, check_name, name)["advice"]
        .as_str()
        .unwrap_or_default()
}

pub(super) fn issues(document: &Value) -> Vec<String> {
    document["needs_attention"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["result"] == "issue")
        .map(|entry| {
            format!(
                "{} {}",
                entry["check"].as_str().unwrap(),
                entry["name"].as_str().unwrap()
            )
        })
        .collect()
}

pub(super) fn seed_seen(sandbox: &Sandbox, gone: bool) -> PathBuf {
    let state = sandbox.path().join("state/vpt");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
    let root = RootDir::open(&state).unwrap();
    let ledger = SqliteLedger::open(&root).unwrap();
    let path = sandbox.path().join("voice-memos/Recordings/seen.m4a");
    ledger
        .record_seen(&SeenRow {
            path: path.clone(),
            file_name: "seen.m4a".into(),
            size: 1,
            mtime: FileTime { secs: 7, nanos: 0 },
            flags: 0,
            first_seen: UtcInstant { secs: 8 },
            last_seen: UtcInstant { secs: 9 },
            deferral_count: 0,
            deferral_reason: None,
            deferred_size: None,
            source_gone_at: gone.then_some(UtcInstant { secs: 10 }),
            recording: None,
        })
        .unwrap();
    path
}

pub(super) fn tripwire(sandbox: &Sandbox, name: &str) -> PathBuf {
    let program = sandbox.path().join("bin").join(name);
    let marker = sandbox.path().join(format!("{name}-ran"));
    std::fs::write(&program, format!("#!/bin/sh\n/usr/bin/touch {marker:?}\n")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    program
}

pub(super) fn assert_never_ran(sandbox: &Sandbox, name: &str) {
    assert!(
        !sandbox.path().join(format!("{name}-ran")).exists(),
        "{name} ran"
    );
}
