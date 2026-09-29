use crate::support::{Sandbox, run, stderr, stdout};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Output;
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::{RecordingLedger, SeenRow};
use vpt_domain::time::{FileTime, UtcInstant};

pub(super) use crate::observations_fixture::snapshot;

pub(super) const NAMES: [&str; 18] = [
    "config",
    "stores",
    "git_tree:audio",
    "git_tree:transcripts",
    "git_tree:analysis",
    "git_tree:briefs",
    "git_tree:engine_outputs",
    "git_tree:drafts",
    "git_tree:released",
    "helper",
    "recordings_dir",
    "symlink",
    "subdirectory:Capture",
    "subdirectory:CaptureRecovery",
    "subdirectory:CloudRecordings_ckAssets",
    "subdirectory:EncryptedCloudRecordings",
    "cleanup_pending",
    "source_gone",
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

pub(super) fn run_json(sandbox: &Sandbox, code: i32) -> Vec<Value> {
    report(
        &run(sandbox
            .vpt()
            .current_dir(sandbox.path())
            .args(["doctor", "--json"])),
        code,
    )
}

pub(super) fn report(output: &Output, code: i32) -> Vec<Value> {
    assert_eq!(output.status.code(), Some(code), "{}", stderr(output));
    let text = if code == 0 {
        assert!(output.stderr.is_empty());
        stdout(output)
    } else {
        assert!(output.stdout.is_empty());
        stderr(output)
    };
    let document: Value = serde_json::from_str(&text).unwrap();
    let body = if code == 0 {
        &document
    } else {
        &document["error"]
    };
    let checks = body["checks"].as_array().unwrap().clone();
    let names: std::collections::BTreeSet<_> = checks
        .iter()
        .map(|check| check["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, NAMES.into_iter().collect());
    assert_eq!(checks.len(), 18);
    let failed = checks.iter().filter(|check| check["ok"] == false).count();
    if code == 0 {
        assert_eq!(failed, 0);
        assert_eq!(
            document,
            json!({"schema":"vpt.result/1","command":"doctor","checks":checks})
        );
    } else {
        assert!(failed > 0);
        assert_eq!(
            document,
            json!({"schema":"vpt.error/1","error":{"kind":"refused","rule":"doctor_checks","message":format!("{failed} checks failed"),"ids":[],"completed":[],"checks":checks}})
        );
    }
    checks
}

pub(super) fn check<'a>(checks: &'a [Value], name: &str) -> &'a Value {
    checks.iter().find(|check| check["name"] == name).unwrap()
}

pub(super) fn detail<'a>(checks: &'a [Value], name: &str) -> &'a str {
    check(checks, name)["detail"].as_str().unwrap()
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
