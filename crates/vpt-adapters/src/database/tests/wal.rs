mod peers;
pub(super) mod scenarios;

use super::children::ChildGuard;
use super::support::root;
use crate::RootDir;
use crate::database::{AccessPolicy, Database, DatabaseError};
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) fn open(root: &RootDir, policy: AccessPolicy) -> Database {
    Database::open(
        root.try_clone().unwrap(),
        Path::new("vpt.db"),
        policy,
        Duration::from_millis(20),
    )
    .unwrap()
}

pub(super) fn sql(db: &mut Database, sql: &str) -> Result<(), DatabaseError> {
    db.with_connection::<_, DatabaseError>(|c| {
        c.execute_batch(sql)?;
        Ok(())
    })
}

pub(super) fn values(db: &mut Database) -> Result<Vec<i64>, DatabaseError> {
    db.with_connection::<_, DatabaseError>(|c| {
        let mut s = c.prepare("SELECT value FROM item ORDER BY value")?;
        Ok(s.query_map([], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?)
    })
}

pub(super) fn initialize(db: &mut Database) {
    phase("seed-start");
    db.with_connection::<_, DatabaseError>(|c| {
        let mode: String = c.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        assert_eq!(mode, "wal", "contained filesystem must support WAL");
        c.execute_batch("PRAGMA wal_autocheckpoint=0; CREATE TABLE item(value INTEGER); INSERT INTO item VALUES(1)")?;
        Ok(())
    }).unwrap();
    phase("seed-ready");
}

pub(super) fn phase(name: &'static str) {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    eprintln!(
        "VPT_PHASE {name} elapsed_ms={}",
        START.get_or_init(Instant::now).elapsed().as_millis()
    );
}

pub(super) fn supervised(scenario: &str) {
    let (temp, _root) = root();
    let started = Instant::now();
    peers::run(scenario, temp.path(), started);
    assert!(started.elapsed() < Duration::from_secs(1));
}

fn finish(child: ChildGuard) {
    let output = child.finish().unwrap();
    assert!(output.status.success(), "{output:?}");
    for line in output
        .stderr
        .split(|byte| *byte == b'\n')
        .filter(|line| line.starts_with(b"VPT_PHASE "))
        .take(16)
    {
        println!("{}", line[..line.len().min(96)].escape_ascii());
    }
}

pub(super) use scenarios::run;

#[test]
fn a_reader_keeps_its_snapshot_while_a_writer_commits() {
    supervised("wal_snapshot");
}
#[test]
fn observation_with_a_live_writer_preserves_every_entry() {
    supervised("wal_live_observation");
}
#[test]
fn observation_after_last_writer_close_preserves_every_entry() {
    supervised("wal_closed_observation");
}
#[test]
fn uninitialized_readonly_retains_slot_locks_for_heap_index_reads() {
    supervised("wal_heap_observation");
}
#[test]
fn missing_companions_are_refused_without_repair() {
    supervised("wal_missing_observation");
}
#[test]
fn recovery_required_observation_preserves_every_entry() {
    supervised("wal_recovery_observation");
}
#[test]
fn native_writer_and_rooted_reader_share_committed_wal_frames() {
    supervised("wal_native_writer");
}
#[test]
fn native_reader_pins_a_snapshot_across_rooted_commits() {
    supervised("wal_native_reader");
}
#[test]
fn native_writer_blocks_a_rooted_wal_transaction_with_a_bounded_wait() {
    supervised("wal_native_contention");
}
#[test]
fn rooted_writer_blocks_a_native_wal_transaction() {
    supervised("wal_rooted_contention");
}
#[test]
fn crash_before_commit_excludes_uncommitted_wal_rows() {
    supervised("wal_crash_uncommitted");
}
#[test]
fn crash_after_commit_preserves_committed_wal_rows() {
    supervised("wal_crash_committed");
}
