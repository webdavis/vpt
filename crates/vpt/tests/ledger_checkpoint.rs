#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "show_list_storage/fixture.rs"]
mod fixture;
mod support;

use fixture::{snapshot, uncheckpointed, write_config};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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

fn json(command: &mut std::process::Command) -> serde_json::Value {
    let output = support::run_with_budget(command, Duration::from_secs(10));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    serde_json::from_str(&stdout(&output)).unwrap()
}

fn ingest(sandbox: &Sandbox, name: &str, captured: i64) -> serde_json::Value {
    let path = sandbox.path().join("voice-memos/Recordings").join(name);
    std::fs::write(&path, m4a(captured, 12, name.as_bytes())).unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(captured as u64))
        .unwrap();
    json(sandbox.vpt().args(["ingest", "-o", "json"]))
}

fn titles(sandbox: &Sandbox) -> Vec<String> {
    let listed = json(sandbox.vpt().args(["list", "-o", "json"]));
    let mut titles: Vec<String> = listed["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["title"].as_str().unwrap_or("").to_owned())
        .collect();
    titles.sort();
    titles
}

fn ids(sandbox: &Sandbox) -> Vec<String> {
    let listed = json(sandbox.vpt().args(["list", "-o", "json"]));
    let mut ids: Vec<String> = listed["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect();
    ids.sort();
    ids
}

fn database_only_copy(from: &Sandbox, name: &str) -> Sandbox {
    let reader = Sandbox::new(name);
    write_config(&reader, "");
    let copy = state(&reader);
    std::fs::create_dir(&copy).unwrap();
    std::fs::set_permissions(&copy, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::copy(state(from).join("vpt.db"), copy.join("vpt.db")).unwrap();
    reader
}

fn wal_is_empty(state: &Path) -> bool {
    std::fs::metadata(state.join("vpt.db-wal")).map_or(true, |wal| wal.len() == 0)
}

fn retitle_in_wal(sandbox: &Sandbox, title: &str) -> rusqlite::Connection {
    uncheckpointed(sandbox, &format!("UPDATE recordings SET title = '{title}'"))
}

#[test]
fn a_writing_command_leaves_the_ledger_in_the_database_file() {
    let sandbox = configured("checkpoint-after-write");
    let ingested = ingest(&sandbox, "a.m4a", 1_787_604_456);
    assert!(wal_is_empty(&state(&sandbox)));
    let reader = database_only_copy(&sandbox, "checkpoint-after-write-copy");
    assert_eq!(
        ids(&reader),
        vec![ingested["ingested"][0]["id"].as_str().unwrap().to_owned()]
    );
}

#[test]
fn a_reader_of_the_database_file_alone_sees_every_recording_and_writes_nothing() {
    let sandbox = configured("checkpoint-database-only");
    ingest(&sandbox, "a.m4a", 1_787_604_456);
    ingest(&sandbox, "b.m4a", 1_787_604_999);
    let reader = database_only_copy(&sandbox, "checkpoint-database-only-copy");
    let before = snapshot(&reader);
    assert_eq!(ids(&reader), ids(&sandbox));
    assert_eq!(ids(&reader).len(), 2);
    assert_eq!(snapshot(&reader), before);
    let leaves: Vec<_> = std::fs::read_dir(state(&reader))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(leaves, vec!["vpt.db"]);
}

#[test]
fn a_ledger_left_in_the_wal_by_a_crash_or_an_older_vpt_is_read_then_moved() {
    let sandbox = configured("checkpoint-left-in-wal");
    ingest(&sandbox, "a.m4a", 1_787_604_456);
    drop(retitle_in_wal(&sandbox, "kept-in-wal"));
    assert_eq!(titles(&sandbox), vec!["kept-in-wal"]);
    ingest(&sandbox, "b.m4a", 1_787_604_999);
    assert!(wal_is_empty(&state(&sandbox)));
    let reader = database_only_copy(&sandbox, "checkpoint-left-in-wal-copy");
    assert_eq!(titles(&reader).len(), 2);
    assert!(titles(&reader).contains(&"kept-in-wal".to_owned()));
}

#[test]
fn a_reader_holding_the_ledger_leaves_the_change_in_the_wal_without_failing_the_writer() {
    let sandbox = configured("checkpoint-held-by-reader");
    ingest(&sandbox, "a.m4a", 1_787_604_456);
    let reader = retitle_in_wal(&sandbox, "kept-in-wal");
    reader.execute_batch("BEGIN").unwrap();
    let held: i64 = reader
        .query_row("SELECT count(*) FROM recordings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(held, 1);
    ingest(&sandbox, "b.m4a", 1_787_604_999);
    assert!(!wal_is_empty(&state(&sandbox)));
    reader.execute_batch("COMMIT").unwrap();
    drop(reader);
    assert_eq!(titles(&sandbox).len(), 2);
    ingest(&sandbox, "c.m4a", 1_787_605_555);
    assert!(wal_is_empty(&state(&sandbox)));
    let copy = database_only_copy(&sandbox, "checkpoint-held-by-reader-copy");
    assert_eq!(ids(&copy), ids(&sandbox));
    assert_eq!(ids(&copy).len(), 3);
}
