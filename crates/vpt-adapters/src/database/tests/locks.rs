use super::children::{CLEANUP_BOUND, child};
use super::support::root;
use crate::database::files::DatabaseFiles;
use crate::database::locks::{DatabaseLocks, LockKind, set_record_lock};
use crate::database::{AccessPolicy, DatabaseError, FileRole};
use rusqlite::ffi::{
    SQLITE_LOCK_EXCLUSIVE as EXCLUSIVE, SQLITE_LOCK_NONE as NONE, SQLITE_LOCK_PENDING as PENDING,
    SQLITE_LOCK_RESERVED as RESERVED, SQLITE_LOCK_SHARED as SHARED,
};
use std::io::ErrorKind;
use std::path::Path;
use std::time::Instant;

fn owners() -> (tempfile::TempDir, DatabaseLocks, DatabaseLocks) {
    let (temp, root) = root();
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
    let first = DatabaseLocks::new(files.open(FileRole::Database, true).unwrap());
    let second = DatabaseLocks::new(files.open(FileRole::Database, false).unwrap());
    (temp, first, second)
}

fn busy(result: Result<(), DatabaseError>) {
    assert!(
        matches!(result, Err(DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(code, _)))
        if code.code == rusqlite::ErrorCode::DatabaseBusy),
        "{result:?}"
    );
}

#[test]
fn closing_another_description_keeps_the_writer_lock() {
    let (temp, _root) = root();
    let output = child("close_description", temp.path())
        .unwrap()
        .finish()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

pub(super) fn close_description(root: crate::RootDir) {
    let files = DatabaseFiles::new(
        root.try_clone().unwrap(),
        Path::new("vpt.db"),
        AccessPolicy::WritableLedger,
    )
    .unwrap();
    let first = DatabaseLocks::new(files.open(FileRole::Database, true).unwrap());
    let second = DatabaseLocks::new(files.open(FileRole::Database, false).unwrap());
    let unrelated = root
        .open_file(Path::new("vpt.db"), crate::Access::ReadWrite)
        .unwrap();
    set_record_lock(first.file(), 0x4000_0001, 1, LockKind::Exclusive).unwrap();
    drop(unrelated);
    busy(set_record_lock(
        second.file(),
        0x4000_0001,
        1,
        LockKind::Exclusive,
    ));
    drop(first);
    set_record_lock(second.file(), 0x4000_0001, 1, LockKind::Exclusive).unwrap();
}

#[test]
fn two_shared_holders_coexist() {
    let (_temp, mut first, mut second) = owners();
    first.lock(SHARED).unwrap();
    second.lock(SHARED).unwrap();
    assert_eq!((first.level(), second.level()), (SHARED, SHARED));
    assert!(!first.reserved().unwrap());
}

#[test]
fn reserved_excludes_a_writer_but_allows_readers() {
    let (_temp, mut first, mut second) = owners();
    first.lock(SHARED).unwrap();
    first.lock(RESERVED).unwrap();
    second.lock(SHARED).unwrap();
    busy(second.lock(RESERVED));
    assert_eq!(second.level(), SHARED);
    assert!(first.reserved().unwrap());
    assert!(second.reserved().unwrap());
}

#[test]
fn pending_blocks_new_readers() {
    let (temp, mut first, mut second) = owners();
    first.lock(SHARED).unwrap();
    second.lock(SHARED).unwrap();
    busy(first.lock(EXCLUSIVE));
    let root = crate::RootDir::open(&temp.path().canonicalize().unwrap().join("state")).unwrap();
    let mut newcomer = DatabaseLocks::new(
        root.open_file(Path::new("vpt.db"), crate::Access::ReadWrite)
            .unwrap(),
    );
    busy(newcomer.lock(SHARED));
    assert_eq!(newcomer.level(), NONE);
}

#[test]
fn failed_promotion_retains_pending_until_readers_leave() {
    let (_temp, mut first, mut second) = owners();
    first.lock(SHARED).unwrap();
    first.lock(RESERVED).unwrap();
    second.lock(SHARED).unwrap();
    busy(first.lock(EXCLUSIVE));
    assert_eq!(first.level(), PENDING);
    busy(first.lock(EXCLUSIVE));
    assert_eq!(first.level(), PENDING);
    second.unlock(NONE).unwrap();
    first.lock(EXCLUSIVE).unwrap();
    assert_eq!(first.level(), EXCLUSIVE);
    busy(second.lock(SHARED));
}

#[test]
fn downgrade_releases_pending_and_reserved_but_keeps_shared() {
    let (_temp, mut first, mut second) = owners();
    first.lock(SHARED).unwrap();
    first.lock(RESERVED).unwrap();
    first.lock(EXCLUSIVE).unwrap();
    first.unlock(SHARED).unwrap();
    assert_eq!(first.level(), SHARED);
    assert!(!first.reserved().unwrap());
    second.lock(SHARED).unwrap();
    second.lock(RESERVED).unwrap();
    busy(second.lock(EXCLUSIVE));
    first.unlock(NONE).unwrap();
    second.lock(EXCLUSIVE).unwrap();
}

#[test]
fn pending_can_downgrade_to_shared() {
    let (_temp, mut first, mut second) = owners();
    first.lock(SHARED).unwrap();
    second.lock(SHARED).unwrap();
    busy(first.lock(EXCLUSIVE));
    first.unlock(SHARED).unwrap();
    assert_eq!(first.level(), SHARED);
    set_record_lock(second.file(), 0x4000_0000, 1, LockKind::Exclusive).unwrap();
}

#[test]
fn unlock_releases_only_its_own_description() {
    let (_temp, mut first, mut second) = owners();
    first.lock(SHARED).unwrap();
    first.lock(RESERVED).unwrap();
    second.lock(SHARED).unwrap();
    second.unlock(NONE).unwrap();
    assert_eq!(second.level(), NONE);
    assert!(second.reserved().unwrap());
    second.lock(SHARED).unwrap();
    busy(second.lock(RESERVED));
    first.unlock(NONE).unwrap();
    assert!(!second.reserved().unwrap());
    second.lock(RESERVED).unwrap();
}

#[test]
fn failed_shared_acquisition_releases_the_pending_byte() {
    let (_temp, mut first, second) = owners();
    set_record_lock(second.file(), 0x4000_0002, 510, LockKind::Exclusive).unwrap();
    busy(first.lock(SHARED));
    assert_eq!(first.level(), NONE);
    set_record_lock(second.file(), 0x4000_0000, 1, LockKind::Exclusive).unwrap();
}

#[test]
fn pending_contention_preserves_the_previous_level() {
    let (_temp, mut first, second) = owners();
    first.lock(SHARED).unwrap();
    first.lock(RESERVED).unwrap();
    set_record_lock(second.file(), 0x4000_0000, 1, LockKind::Shared).unwrap();
    busy(first.lock(EXCLUSIVE));
    assert_eq!(first.level(), RESERVED);
    set_record_lock(second.file(), 0x4000_0000, 1, LockKind::Unlock).unwrap();
    first.lock(EXCLUSIVE).unwrap();
}

#[test]
fn invalid_lock_levels_and_ranges_do_not_acquire_locks() {
    let (_temp, mut first, mut second) = owners();
    for level in [NONE, PENDING, -1, 5, RESERVED, EXCLUSIVE] {
        assert!(matches!(
            first.lock(level),
            Err(DatabaseError::Io {
                kind: ErrorKind::InvalidInput,
                ..
            })
        ));
        assert_eq!(first.level(), NONE);
    }
    for level in [RESERVED, PENDING, EXCLUSIVE, -1, 5] {
        assert!(matches!(
            first.unlock(level),
            Err(DatabaseError::Io {
                kind: ErrorKind::InvalidInput,
                ..
            })
        ));
    }
    for (start, length) in [(-1, 1), (0, -1), (i64::MAX, 1)] {
        assert!(matches!(
            set_record_lock(first.file(), start, length, LockKind::Exclusive),
            Err(DatabaseError::Io {
                kind: ErrorKind::InvalidInput,
                ..
            })
        ));
    }
    second.lock(SHARED).unwrap();
    second.lock(EXCLUSIVE).unwrap();
}

#[test]
fn repeated_requests_preserve_the_stronger_lock() {
    let (_temp, mut first, mut second) = owners();
    first.unlock(SHARED).unwrap();
    assert_eq!(first.level(), NONE);
    first.lock(SHARED).unwrap();
    first.lock(EXCLUSIVE).unwrap();
    for level in [SHARED, RESERVED, EXCLUSIVE] {
        first.lock(level).unwrap();
        assert_eq!(first.level(), EXCLUSIVE);
        busy(second.lock(SHARED));
    }
    first.unlock(NONE).unwrap();
    first.unlock(NONE).unwrap();
    second.lock(SHARED).unwrap();
}

#[test]
fn child_watchdog_reaps_a_stalled_lock_test() {
    let (temp, _root) = root();
    let started = Instant::now();
    let process = child("stall", temp.path()).unwrap();
    let error = process.finish().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::TimedOut);
    assert!(started.elapsed() < CLEANUP_BOUND);
    let message = error.to_string();
    for expected in [
        "child=running",
        "stdout_eof=false",
        "stderr_eof=false",
        "phase=stall-wait",
        "OUT\\t\\x1b",
        "ERR\\t\\x1b",
    ] {
        assert!(message.contains(expected), "{message}");
    }
    for stream in ["stdout_bytes=", "stderr_bytes="] {
        let count = message
            .split_whitespace()
            .find_map(|part| part.strip_prefix(stream))
            .unwrap();
        assert!(count.parse::<usize>().unwrap() > 1024);
    }
    assert!(message.len() < 1024, "bounded previews: {}", message.len());
}

fn fifo(scenario: &str) {
    let (temp, _root) = root();
    let started = Instant::now();
    let output = child(scenario, temp.path()).unwrap().finish().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(started.elapsed() < CLEANUP_BOUND);
}

#[test]
fn fifo_database() {
    fifo("fifo_database");
}

#[test]
fn fifo_journal() {
    fifo("fifo_journal");
}

#[test]
fn fifo_wal() {
    fifo("fifo_wal");
}

#[test]
fn fifo_shared_memory() {
    fifo("fifo_shared_memory");
}

#[test]
fn native_writer_blocks_the_rooted_contender() {
    let (temp, root) = root();
    let mut native = child("native_writer", temp.path()).unwrap();
    native.ready().unwrap();
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
    let mut rooted = DatabaseLocks::new(files.open(FileRole::Database, false).unwrap());
    busy(rooted.lock(SHARED));
    native.send("continue").unwrap();
    let output = native.finish().unwrap();
    assert!(output.status.success(), "{output:?}");
    rooted.lock(SHARED).unwrap();
    rooted.lock(EXCLUSIVE).unwrap();
}

#[test]
fn rooted_writer_blocks_the_native_contender() {
    let (temp, root) = root();
    let mut native = child("native_contender", temp.path()).unwrap();
    native.ready().unwrap();
    let files =
        DatabaseFiles::new(root, Path::new("vpt.db"), AccessPolicy::WritableLedger).unwrap();
    let mut rooted = DatabaseLocks::new(files.open(FileRole::Database, false).unwrap());
    rooted.lock(SHARED).unwrap();
    rooted.lock(RESERVED).unwrap();
    native.send("continue").unwrap();
    let output = native.finish().unwrap();
    assert!(output.status.success(), "{output:?}");
}
