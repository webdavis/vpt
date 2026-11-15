#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
mod support;

use fixture::{snapshot, write_config};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;
use support::{Sandbox, stderr, stdout};
use vpt_domain::fixtures::m4a;

fn configured(name: &str) -> Sandbox {
    let sandbox = Sandbox::new(name);
    write_config(
        &sandbox,
        &format!(
            "[source]\nrecordings_dir = {:?}\nquiet_period_secs = 1\n",
            sandbox.path().join("voice-memos/Recordings")
        ),
    );
    sandbox
}

fn state(sandbox: &Sandbox) -> PathBuf {
    sandbox.path().join("state/vpt")
}

fn ingest(sandbox: &Sandbox, name: &str, captured: i64) -> String {
    let path = sandbox.path().join("voice-memos/Recordings").join(name);
    std::fs::write(&path, m4a(captured, 12, name.as_bytes())).unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(captured as u64))
        .unwrap();
    let output = support::run_with_budget(
        sandbox.vpt().args(["ingest", "--json"]),
        Duration::from_secs(10),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    document["ingested"][0]["id"].as_str().unwrap().to_owned()
}

fn database_only_copy(from: &Sandbox, name: &str) -> Sandbox {
    let reader = configured(name);
    let copy = state(&reader);
    std::fs::create_dir(&copy).unwrap();
    std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::copy(state(from).join("vpt.db"), copy.join("vpt.db")).unwrap();
    reader
}

fn leaves(sandbox: &Sandbox) -> Vec<std::ffi::OsString> {
    let mut leaves: Vec<_> = std::fs::read_dir(state(sandbox))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    leaves.sort();
    leaves
}

fn reads_without_writing(sandbox: &Sandbox, id: &str) {
    let before = snapshot(sandbox);
    let commands: [&[&str]; 8] = [
        &["doctor"],
        &["show", id],
        &["list"],
        &["list", "--json"],
        &["storage"],
        &["ledger", "show"],
        &["ingest", "--dry-run"],
        &["run", "--dry-run"],
    ];
    for args in commands {
        let output = support::run_with_budget(sandbox.vpt().args(args), Duration::from_secs(10));
        assert!(output.status.code().is_some(), "{args:?} ended by a signal");
        assert_eq!(snapshot(sandbox), before, "{args:?} changed a file");
    }
    let shown = support::run_with_budget(sandbox.vpt().args(["show", id]), Duration::from_secs(10));
    assert_eq!(shown.status.code(), Some(0), "{}", stderr(&shown));
}

#[test]
fn read_only_commands_change_nothing_on_a_healthy_ledger_or_a_database_file_alone() {
    let sandbox = configured("read-only-state");
    let id = ingest(&sandbox, "a.m4a", 1_787_604_456);
    ingest(&sandbox, "b.m4a", 1_787_604_999);
    let healthy = leaves(&sandbox);
    assert!(healthy.iter().any(|leaf| leaf == "vpt.db"), "{healthy:?}");
    assert!(
        !healthy.iter().any(|leaf| leaf == "vpt.db-wal"),
        "{healthy:?}"
    );
    reads_without_writing(&sandbox, &id);
    let copy = database_only_copy(&sandbox, "read-only-state-copy");
    assert_eq!(leaves(&copy), vec!["vpt.db"]);
    reads_without_writing(&copy, &id);
}
