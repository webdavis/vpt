#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "ingest/abort.rs"]
mod abort;
#[path = "ingest/absent_source.rs"]
mod absent_source;
#[path = "ingest/capture_time.rs"]
mod capture_time;
#[path = "ingest/config_refusal.rs"]
mod config_refusal;
#[path = "ingest/deferrals.rs"]
mod deferrals;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "ingest/identity.rs"]
mod identity;
#[path = "ingest/logging.rs"]
mod logging;
#[path = "ingest/missing_parents.rs"]
mod missing_parents;
#[path = "ingest/output.rs"]
mod output;
#[path = "ingest/reporting.rs"]
mod reporting;
#[path = "ingest/selection.rs"]
mod selection;
#[path = "ingest/startup.rs"]
mod startup;
mod support;

use fixture::{snapshot, write_config};
use support::{Sandbox, stderr, stdout};
use vpt_domain::fixtures::m4a;

fn run(command: &mut std::process::Command) -> std::process::Output {
    support::run_with_budget(command, std::time::Duration::from_secs(3))
}

fn configured() -> Sandbox {
    let sandbox = Sandbox::new("ingest-command");
    write_config(
        &sandbox,
        &format!(
            "[source]\nrecordings_dir = {:?}\nquiet_period_secs = 1\n",
            sandbox.path().join("voice-memos/Recordings")
        ),
    );
    sandbox
}

fn add(sandbox: &Sandbox, name: &str) -> std::path::PathBuf {
    let path = sandbox.path().join("voice-memos/Recordings").join(name);
    std::fs::write(&path, m4a(1_787_604_456, 12, b"private audio")).unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_787_604_456))
        .unwrap();
    path
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(output.stderr.is_empty());
    serde_json::from_str(&stdout(output)).unwrap()
}

#[test]
fn dry_run_with_absent_state_is_observational_and_reports_candidates() {
    let sandbox = configured();
    let path = add(&sandbox, "a.m4a");
    let before = snapshot(&sandbox);
    let doc = json(&run(sandbox.vpt().args(["ingest", "--dry-run", "--json"])));
    assert_eq!(doc["command"], "ingest");
    assert_eq!(doc["would_ingest"][0]["path"], path.to_str().unwrap());
    assert_eq!(doc["would_ingest"][0]["title_source"], "unavailable");
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn ingest_archives_records_logs_and_skips_an_unchanged_source() {
    let sandbox = configured();
    let path = add(&sandbox, "a.m4a");
    let first = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    let id = first["ingested"][0]["id"].as_str().unwrap();
    let audio = first["ingested"][0]["audio_path"].as_str().unwrap();
    assert_eq!(std::fs::read(audio).unwrap(), std::fs::read(path).unwrap());
    let log = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(log.contains("ingest"));
    assert!(!log.contains("private audio"));
    let second = json(&run(sandbox.vpt().args(["ingest", "--json"])));
    assert_eq!(second["skipped"], 1);
    assert_eq!(second["already_ingested"][0], id);
    assert_eq!(second["ingested"], serde_json::json!([]));
}

#[test]
fn once_dry_run_refuses_an_escaped_source() {
    let sandbox = configured();
    let outside = sandbox.path().join("outside.m4a");
    std::fs::write(&outside, m4a(1_787_604_456, 1, b"outside")).unwrap();
    let before = snapshot(&sandbox);
    let output = run(sandbox
        .vpt()
        .args(["ingest", "--dry-run", "--json", "--once"])
        .arg(outside));
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let doc: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(doc["error"]["rule"], "path_escape");
    assert_eq!(snapshot(&sandbox), before);
}
