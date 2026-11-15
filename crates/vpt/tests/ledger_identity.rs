#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "ledger_identity/accept.rs"]
mod accept;
#[path = "ledger_identity/accepted.rs"]
mod accepted;
#[path = "ledger_identity/doctor.rs"]
mod doctor;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "ledger_identity/read_only.rs"]
mod read_only;
#[path = "ledger_identity/refusals.rs"]
mod refusals;
#[path = "ledger_identity/show.rs"]
mod show;
mod support;

use fixture::write_config;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;
use support::{Sandbox, run_with_budget, stderr, stdout};
use vpt_domain::fixtures::m4a;

/// One machine: its own home, its own state folder, and so its own marker.
pub(crate) fn machine(name: &str) -> Sandbox {
    let sandbox = Sandbox::new(name);
    point_at(&sandbox, &state(&sandbox));
    sandbox
}

pub(crate) fn point_at(sandbox: &Sandbox, folder: &Path) {
    write_config(
        sandbox,
        &format!(
            "[source]\nrecordings_dir = {:?}\nquiet_period_secs = 1\n",
            sandbox.path().join("voice-memos/Recordings")
        ),
    );
    let config = std::fs::read_to_string(sandbox.config_path()).unwrap();
    let default = format!("{:?}", state(sandbox));
    std::fs::write(
        sandbox.config_path(),
        config.replace(&default, &format!("{folder:?}")),
    )
    .unwrap();
}

pub(crate) fn state(sandbox: &Sandbox) -> PathBuf {
    sandbox.path().join("state/vpt")
}

pub(crate) fn vpt(sandbox: &Sandbox, args: &[&str]) -> Output {
    run_with_budget(sandbox.vpt().args(args), Duration::from_secs(10))
}

pub(crate) fn succeeds(sandbox: &Sandbox, args: &[&str]) -> Output {
    let output = vpt(sandbox, args);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    output
}

pub(crate) fn json(sandbox: &Sandbox, args: &[&str]) -> serde_json::Value {
    serde_json::from_str(&stdout(&succeeds(sandbox, args))).unwrap()
}

/// Leaves a new recording in the machine's Voice Memos folder, ready for the next ingest.
pub(crate) fn record(sandbox: &Sandbox, name: &str, captured: i64) {
    let path = sandbox.path().join("voice-memos/Recordings").join(name);
    std::fs::write(&path, m4a(captured, 12, name.as_bytes())).unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(captured as u64))
        .unwrap();
}

pub(crate) fn ingest(sandbox: &Sandbox, name: &str, captured: i64) -> Output {
    record(sandbox, name, captured);
    vpt(sandbox, &["ingest"])
}

pub(crate) fn ingested(sandbox: &Sandbox, name: &str, captured: i64) {
    let output = ingest(sandbox, name, captured);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(!stderr(&output).contains("ledger_"), "{}", stderr(&output));
}

pub(crate) fn recordings(sandbox: &Sandbox) -> usize {
    json(sandbox, &["list", "--json"])["recordings"]
        .as_array()
        .unwrap()
        .len()
}

/// Plays a sync service: the database file arrives whole in the other machine's folder.
pub(crate) fn deliver(database: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    std::fs::set_permissions(to, std::fs::Permissions::from_mode(0o700)).unwrap();
    for companion in ["vpt.db-wal", "vpt.db-shm"] {
        let _ = std::fs::remove_file(to.join(companion));
    }
    let arriving = to.join(".vpt.db.arriving");
    std::fs::copy(database, &arriving).unwrap();
    std::fs::rename(arriving, to.join("vpt.db")).unwrap();
}

pub(crate) fn backup(sandbox: &Sandbox) -> PathBuf {
    let copy = sandbox.path().join(format!(
        "backup-{}.db",
        std::fs::read_dir(sandbox.path()).unwrap().count()
    ));
    std::fs::copy(state(sandbox).join("vpt.db"), &copy).unwrap();
    copy
}

pub(crate) fn assert_refused(output: &Output, rule: &str, folder: &Path) {
    assert_eq!(output.status.code(), Some(3), "{}", stderr(output));
    let text = stderr(output);
    assert!(text.starts_with(&format!("vpt: {rule}: ")), "{text}");
    assert!(text.contains(&folder.display().to_string()), "{text}");
    assert!(text.contains("`vpt ledger show`"), "{text}");
    assert!(text.trim_end().ends_with("vpt ledger accept"), "{text}");
    assert!(output.stdout.is_empty());
}
