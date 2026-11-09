#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "retention/failures.rs"]
mod failures;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "retention/output.rs"]
mod output;
#[path = "retention/recovery.rs"]
mod recovery;
#[path = "retention/reporting.rs"]
mod reporting;
mod support;

use fixture::{record, seed, snapshot, write_config};
use std::time::{Duration, SystemTime};
use support::{Sandbox, run, stderr, stdout};
use vpt_adapters::{RootDir, SqliteLedger, WriteLock, digest_open};
use vpt_application::ports::{RecordingLedger, StageStates};

fn configured(enabled: bool) -> Sandbox {
    let sandbox = Sandbox::new("retention-command");
    write_config(
        &sandbox,
        &format!(
            "[source]\nrecordings_dir = {:?}\n[retention]\nenabled = {enabled}\ninclude_audio = true\n[retention.hold]\naudio = \"1d\"\n",
            sandbox.path().join("voice-memos/Recordings")
        ),
    );
    sandbox
}

fn expired_audio(sandbox: &Sandbox) -> vpt_application::ports::RecordingRecord {
    let mut row = record(sandbox, 1, StageStates::fresh());
    std::fs::create_dir_all(row.audio_path.parent().unwrap()).unwrap();
    std::fs::write(&row.audio_path, b"owned audio").unwrap();
    let mut file = std::fs::File::open(&row.audio_path).unwrap();
    row.digest = digest_open(&mut file).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(100))
        .unwrap();
    seed(sandbox, vec![row.clone()]);
    row
}

#[test]
fn disabled_retention_is_a_configuration_error_without_writes() {
    let sandbox = configured(false);
    let before = snapshot(&sandbox);
    let output = run(sandbox.vpt().args(["retention", "run", "--json"]));
    let doc = error(&output, 2, "config");
    assert_eq!(doc["error"]["message"], "retention is disabled");
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn dry_run_with_absent_state_creates_nothing() {
    let sandbox = configured(true);
    let before = snapshot(&sandbox);
    let doc = json(&run(sandbox.vpt().args([
        "retention",
        "run",
        "--dry-run",
        "--json",
    ])));
    assert_eq!(doc["command"], "retention run");
    assert_eq!(doc["moved"], serde_json::json!([]));
    assert_eq!(doc["dry_run"], true);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn dry_run_reports_candidates_while_write_lock_is_held_without_changes() {
    let sandbox = configured(true);
    let row = expired_audio(&sandbox);
    let root = RootDir::open(&sandbox.path().join("state/vpt")).unwrap();
    let _lock = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let before = snapshot(&sandbox);
    let doc = json(&run(sandbox.vpt().args([
        "retention",
        "run",
        "--dry-run",
        "--json",
    ])));
    assert_eq!(doc["moved"][0]["path"], row.audio_path.to_str().unwrap());
    assert_eq!(doc["counts"]["audio"], 1);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn retention_moves_owned_audio_and_logs_the_result() {
    let sandbox = configured(true);
    sandbox.install_fake_helper();
    let row = expired_audio(&sandbox);
    let doc = json(&run(sandbox.vpt().args(["retention", "run", "--json"])));
    assert_eq!(doc["moved"][0]["store"], "audio");
    assert_eq!(doc["completed"], serde_json::json!([row.id.as_str()]));
    assert!(!row.audio_path.exists());
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(
        ledger
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .audio_trashed_at
            .is_some()
    );
    let log = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(log.contains("retention"));
    assert!(!log.contains("title-1"));
}

#[test]
fn missing_helper_refuses_removal_and_preserves_audio() {
    let sandbox = configured(true);
    let row = expired_audio(&sandbox);
    let doc = error(
        &run(sandbox.vpt().args(["retention", "run", "--json"])),
        3,
        "refused",
    );
    assert_eq!(doc["error"]["rule"], "helper_absent");
    assert_eq!(std::fs::read(row.audio_path).unwrap(), b"owned audio");
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).expect("one result document")
}

fn error(output: &std::process::Output, code: i32, kind: &str) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(code), "{}", stderr(output));
    assert!(output.stdout.is_empty());
    let document: serde_json::Value =
        serde_json::from_str(&stderr(output)).expect("one error document");
    assert_eq!(document["schema"], "vpt.error/1");
    assert_eq!(document["error"]["kind"], kind);
    document
}
