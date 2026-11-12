use crate::fixture::{configured, detail, run_json, seed_seen, snapshot};
use crate::support;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn writer_child() {
    let Some(state) = std::env::var_os("VPT_TEST_DOCTOR_WRITER_STATE") else {
        return;
    };
    let state = std::path::PathBuf::from(state);
    let connection = rusqlite::Connection::open_with_flags(
        state.join("vpt.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .unwrap();
    crate::observations_fixture::preserve_wal(&connection);
    connection.execute_batch("BEGIN IMMEDIATE").unwrap();
    let source = std::env::var("VPT_TEST_DOCTOR_WRITER_SOURCE").unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE seen SET source_gone_at = ?1 WHERE path = ?2",
                rusqlite::params![321_i64, source]
            )
            .unwrap(),
        1
    );
    std::fs::write(state.join("writer-ready"), b"ready").unwrap();
    let started = Instant::now();
    while !state.join("writer-release").exists() {
        assert!(
            started.elapsed() < support::WATCHDOG,
            "writer release timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    connection.execute_batch("ROLLBACK").unwrap();
}

#[test]
fn doctor_reads_committed_seen_rows_during_native_write() {
    let sandbox = configured("doctor-active-writer");
    let source = seed_seen(&sandbox, false);
    let state = sandbox.path().join("state/vpt");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "active_writer::writer_child", "--nocapture"])
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
        .env("VPT_TEST_DOCTOR_WRITER_STATE", &state)
        .env("VPT_TEST_DOCTOR_WRITER_SOURCE", source);
    let started = Instant::now();
    let mut child = support::spawn(&mut command);
    while !crate::observations_fixture::writer_ready(&state) {
        assert!(
            started.elapsed() < support::WATCHDOG,
            "writer readiness timed out"
        );
        assert!(
            child.observe().unwrap().is_none(),
            "writer exited before readiness"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 0);
    assert_eq!(
        detail(&checks, "source_gone"),
        "0 recordings whose source is gone"
    );
    assert_eq!(snapshot(&sandbox), before);
    std::fs::write(state.join("writer-release"), b"release").unwrap();
    let output = support::collect(child, started, None);
    assert!(output.status.success(), "{}", support::stderr(&output));
}
