use super::fixture::{Fixture, titled, unchanged};
use super::*;
use crate::Access;
use crate::record_lock::{LockKind, set_record_lock};
use crate::test_support::{child_for_root, root, snapshot};
use std::process::Output;
use std::time::Instant;

const INDEX_LIFETIME_BYTE: i64 = 128;

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        output.stdout.escape_ascii(),
        output.stderr.escape_ascii()
    );
}

#[test]
fn reading_a_title_keeps_the_record_locks_sqlite_holds_on_the_copy() {
    super::bounded("title_held_locks");
}

#[test]
fn a_container_another_process_holds_open_is_read_through_a_copy_and_never_changed() {
    let (temp, _root) = root();
    let spawn =
        |scenario| child_for_root(scenario, temp.path(), std::process::id(), Instant::now());
    let mut holder = spawn("title_held_container").unwrap();
    holder.ready().unwrap();
    let reader = spawn("title_held_reader").and_then(|child| child.finish());
    let released = holder.send("continue");
    let holder = holder.finish().unwrap();
    released.unwrap();
    success(&reader.unwrap());
    success(&holder);
}

pub(super) fn run(scenario: &str, root: &RootDir, ready: fn()) {
    match scenario {
        "title_held_locks" => locks(root),
        "title_held_container" => {
            let fixture = Fixture::new(root, &[("a.m4a", Some("A")), ("b.m4a", Some("B"))]);
            fixture
                .source
                .execute_batch("PRAGMA wal_autocheckpoint=0; UPDATE ZCLOUDRECORDING SET ZCUSTOMLABEL='B2' WHERE ZPATH='b.m4a'")
                .unwrap();
            ready();
        }
        "title_held_reader" => {
            let container = root.path().join("source");
            let state = root.open_subdirectory("private").unwrap();
            let before = snapshot(&container);
            assert!(
                std::fs::metadata(container.join("CloudRecordings.db-wal"))
                    .unwrap()
                    .len()
                    > 0
            );
            let copy = TitleCopy::refresh(&container, state.path()).unwrap();
            titled(&copy, "a.m4a", "A");
            titled(&copy, "b.m4a", "B2");
            drop(copy);
            unchanged(&before, &snapshot(&container));
        }
        _ => panic!("unknown held title scenario"),
    }
}

fn locks(root: &RootDir) {
    let fixture = Fixture::simple(root);
    let copy = fixture.refresh();
    titled(&copy, "a.m4a", "A");
    let directory = fixture.state.open_subdirectory(COPY_DIRECTORY).unwrap();
    let probe = directory
        .open_file(Path::new("CloudRecordings.db-shm"), Access::ReadWrite)
        .unwrap();
    let held = || set_record_lock(&probe, INDEX_LIFETIME_BYTE, 1, LockKind::Exclusive).is_err();
    assert!(held(), "the copy's connection holds its index lock");
    titled(&copy, "a.m4a", "A");
    assert!(held(), "reading a title left the index lock in place");
    drop(copy);
    assert!(!held(), "closing the copy released its index lock");
}
