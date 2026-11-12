use super::{configured, json, record, run, seed, snapshot, stderr, support};
use crate::fixture::preserve_wal;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use vpt_application::ports::StageStates;

#[test]
fn sqlite_writer_child() {
    let Some(state) = std::env::var_os("VPT_TEST_SQLITE_WRITER_STATE") else {
        return;
    };
    let state = std::path::PathBuf::from(state);
    let connection = rusqlite::Connection::open_with_flags(
        state.join("vpt.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .unwrap();
    preserve_wal(&connection);
    connection.execute_batch("BEGIN IMMEDIATE").unwrap();
    let id = std::env::var("VPT_TEST_SQLITE_WRITER_ID").unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE recordings SET title = ?1 WHERE id = ?2",
                rusqlite::params!["uncommitted-canary", id],
            )
            .unwrap(),
        1
    );
    let ready = state.join("writer-ready");
    let release = state.join("writer-release");
    std::fs::write(&ready, b"ready").unwrap();
    let started = Instant::now();
    while !release.exists() {
        assert!(
            started.elapsed() < crate::support::WATCHDOG,
            "writer release timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    connection.execute_batch("ROLLBACK").unwrap();
}

#[test]
fn show_and_list_read_committed_rows_while_another_process_writes() {
    let sandbox = configured("observations-active-sqlite-writer");
    let row = record(&sandbox, 1, StageStates::fresh());
    seed(&sandbox, vec![row.clone()]);
    let state = sandbox.path().join("state/vpt");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "active_writer::sqlite_writer_child",
            "--nocapture",
        ])
        .current_dir(sandbox.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("HOME", sandbox.path().join("home"))
        .env("XDG_CONFIG_HOME", sandbox.path().join("config"))
        .env("XDG_DATA_HOME", sandbox.path().join("data"))
        .env("XDG_STATE_HOME", sandbox.path().join("state"))
        .env("XDG_CACHE_HOME", sandbox.path().join("cache"))
        .env("TMPDIR", sandbox.path().join("tmp"))
        .env("VPT_CONFIG", sandbox.config_path())
        .env("PATH", sandbox.path().join("bin"))
        .env("VPT_TEST_SQLITE_WRITER_STATE", &state)
        .env("VPT_TEST_SQLITE_WRITER_ID", row.id.as_str());
    let started = Instant::now();
    let mut child = support::spawn(&mut command);
    let deadline = started + crate::support::WATCHDOG;
    while !crate::fixture::writer_ready(&state) {
        assert!(
            Instant::now() < deadline,
            "writer child did not become ready"
        );
        assert!(
            child.observe().unwrap().is_none(),
            "writer child exited before ready"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let before = snapshot(&sandbox);
    let mut show = sandbox.vpt();
    show.current_dir(sandbox.path())
        .args(["show", row.id.as_str(), "--json"]);
    let shown = json(&run(&mut show));
    assert_eq!(shown["id"], row.id.as_str());
    assert_eq!(shown["title"], "title-1");
    let mut list = sandbox.vpt();
    list.current_dir(sandbox.path()).args(["list", "--json"]);
    let listed = json(&run(&mut list));
    assert_eq!(listed["recordings"][0]["id"], row.id.as_str());
    assert_eq!(listed["recordings"][0]["title"], "title-1");
    assert_eq!(snapshot(&sandbox), before);
    std::fs::write(state.join("writer-release"), b"release").unwrap();
    let output = support::collect(child, started, None);
    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn writer_readiness_waits_for_complete_marker() {
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("writer-ready");
    assert!(!crate::fixture::writer_ready(state.path()));
    std::fs::write(&marker, b"").unwrap();
    assert!(!crate::fixture::writer_ready(state.path()));
    std::fs::write(&marker, b"rea").unwrap();
    assert!(!crate::fixture::writer_ready(state.path()));
    std::fs::write(&marker, b"ready").unwrap();
    assert!(crate::fixture::writer_ready(state.path()));
}
