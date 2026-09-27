mod process;
pub(crate) mod scenarios;

use super::*;
use crate::database::{ChildGuard, child_for_root, test_root};
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

fn child(scenario: &str, directory: &std::path::Path, started: Instant) -> ChildGuard {
    child_for_root(scenario, directory, std::process::id(), started).expect("owned lock child")
}

fn finished(child: ChildGuard) {
    let output = child.finish().expect("bounded lock child");
    assert!(
        output.status.success(),
        "lock child failed: stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
}

#[test]
fn an_uncontended_lock_accepts_duration_max_promptly() {
    let (temp, _state) = test_root();
    finished(child("lock_max", temp.path(), Instant::now()));
}

#[test]
fn a_held_lock_makes_zero_wait_busy() {
    let (temp, _state) = test_root();
    finished(child("lock_zero_self", temp.path(), Instant::now()));
}

#[test]
fn a_held_lock_waits_within_the_requested_bound() {
    let (temp, _state) = test_root();
    finished(child("lock_bounded_self", temp.path(), Instant::now()));
}

#[test]
fn dropping_the_owner_releases_the_lock() {
    let (temp, _state) = test_root();
    finished(child("lock_drop", temp.path(), Instant::now()));
}

#[test]
fn a_new_lock_file_is_private() {
    let (temp, state) = test_root();
    finished(child("lock_private", temp.path(), Instant::now()));
    let mode = fs::metadata(state.path().join("write.lock"))
        .expect("lock file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn the_lock_descriptor_closes_on_exec() {
    let (temp, _state) = test_root();
    finished(child("lock_cloexec", temp.path(), Instant::now()));
}

#[test]
fn a_failed_flock_reports_io() {
    let (temp, _state) = test_root();
    finished(child("lock_io", temp.path(), Instant::now()));
}

#[test]
fn a_link_at_the_lock_name_is_refused() {
    let (temp, state) = test_root();
    let elsewhere = temp.path().join("elsewhere");
    fs::write(&elsewhere, b"").expect("target");
    std::os::unix::fs::symlink(&elsewhere, state.path().join("write.lock")).expect("owned link");
    finished(child("lock_symlink", temp.path(), Instant::now()));
}

#[test]
fn a_directory_at_the_lock_name_is_refused() {
    let (temp, state) = test_root();
    fs::create_dir(state.path().join("write.lock")).expect("owned directory");
    finished(child("lock_directory", temp.path(), Instant::now()));
}

#[test]
fn a_fifo_at_the_lock_name_is_refused_without_blocking() {
    let (temp, state) = test_root();
    let path = state.path().join("write.lock");
    let name = CString::new(path.as_os_str().as_bytes()).expect("FIFO path");
    // SAFETY: name is an owned NUL-terminated path to this private fixture.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    finished(child("lock_fifo", temp.path(), Instant::now()));
}

#[test]
fn a_hard_link_at_the_lock_name_is_refused() {
    let (temp, state) = test_root();
    let source = state.path().join("source");
    fs::write(&source, b"").expect("owned source");
    fs::hard_link(&source, state.path().join("write.lock")).expect("owned hard link");
    finished(child("lock_hardlink", temp.path(), Instant::now()));
}
