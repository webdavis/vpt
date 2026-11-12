use super::identity::terminal;
use super::supervision::assert_reaped;
use super::{CLEANUP_BOUND, ChildGuard, WATCHDOG, process, spawn_with_signal};
use crate::database::tests::support::root;
use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const HISTORY_EXIT: &str = "ledger_contract_history_budget_exit";
const HISTORY_STALL: &str = "ledger_contract_history_budget_stall";
type SignalRecord = (i32, Result<Option<(i32, i32)>, i32>);

fn monotonic() -> Duration {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: this fixture supplies initialized writable storage for the monotonic clock.
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) },
        0
    );
    Duration::new(
        time.tv_sec.try_into().unwrap(),
        time.tv_nsec.try_into().unwrap(),
    )
}

fn participant(
    scenario: &str,
    directory: &Path,
    started: Instant,
) -> (ChildGuard, Receiver<SignalRecord>) {
    let fallback = CLEANUP_BOUND.saturating_sub(Duration::from_millis(50));
    let remaining = fallback.saturating_sub(started.elapsed());
    std::fs::write(
        directory.join("budget-fallback"),
        (monotonic() + remaining).as_nanos().to_string(),
    )
    .unwrap();
    let cleanup = started + CLEANUP_BOUND;
    let (send, receive) = mpsc::channel();
    let signal = move |pid, signal| {
        process::signal_group(pid, signal);
        let mut observed = terminal(pid);
        while signal == libc::SIGKILL && observed == Ok(None) && Instant::now() < cleanup {
            std::thread::sleep(Duration::from_millis(1));
            observed = terminal(pid);
        }
        let _ = send.send((signal, observed));
    };
    let child = spawn_with_signal(
        scenario,
        directory,
        Some(std::process::id().to_string().into()),
        started,
        signal,
    )
    .unwrap();
    (child, receive)
}

#[test]
fn history_child_preserves_original_operation_and_cleanup_anchors() {
    let (directory, _root) = root();
    let started = Instant::now() - Duration::from_millis(200);
    let (child, _records) = participant(HISTORY_EXIT, directory.path(), started);
    let pid = child.group.id();
    let operation = child.deadline;
    let cleanup = child.group.cleanup_deadline();
    drop(child);
    assert_reaped(pid);
    assert_eq!(operation, started + WATCHDOG);
    assert_eq!(cleanup, started + CLEANUP_BOUND);
}

#[test]
fn ordinary_child_preserves_original_operation_and_cleanup_anchors() {
    let (directory, _root) = root();
    let started = Instant::now() - Duration::from_millis(200);
    let (child, _records) = participant("budget_exit", directory.path(), started);
    let pid = child.group.id();
    let operation = child.deadline;
    let cleanup = child.group.cleanup_deadline();
    drop(child);
    assert_reaped(pid);
    assert_eq!(operation, started + WATCHDOG);
    assert_eq!(cleanup, started + CLEANUP_BOUND);
}

#[test]
fn finite_history_participant_reports_success() {
    let (directory, _root) = root();
    let started = Instant::now();
    let (child, _records) = participant(HISTORY_EXIT, directory.path(), started);
    let pid = child.group.id();
    let output = child.finish();
    assert_reaped(pid);
    assert!(started.elapsed() < CLEANUP_BOUND);
    assert!(output.unwrap().status.success());
}

#[test]
fn stalled_history_participant_is_force_killed_and_reaped_within_cleanup_bound() {
    let (directory, _root) = root();
    let started = Instant::now();
    let (mut child, records) = participant(HISTORY_STALL, directory.path(), started);
    let pid = child.group.id();
    let ready = child.ready();
    let output = child.finish();
    let elapsed = started.elapsed();
    assert_reaped(pid);
    assert!(elapsed < CLEANUP_BOUND, "cleanup took {elapsed:?}");
    ready.expect("stall participant installed TERM refusal");
    let error = output.expect_err("a stalled participant must time out");
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("phase=history-budget-stall"));
    let records: Vec<_> = records.try_iter().collect();
    assert_eq!(
        records,
        [
            (libc::SIGTERM, Ok(None)),
            (libc::SIGKILL, Ok(Some((libc::CLD_KILLED, libc::SIGKILL)))),
        ]
    );
    let reported = error.to_string();
    let elapsed_ms = reported
        .split_ascii_whitespace()
        .find_map(|field| field.strip_prefix("parent_elapsed_ms="))
        .unwrap()
        .parse::<u128>()
        .unwrap();
    assert!(
        elapsed_ms >= WATCHDOG.as_millis(),
        "history timed out early: {reported}"
    );
}

pub(super) fn finite_child(scenario: &str, directory: &Path) -> ! {
    if scenario != HISTORY_STALL {
        std::process::exit(0);
    }
    let fallback: u128 = std::fs::read_to_string(directory.join("budget-fallback"))
        .unwrap()
        .parse()
        .unwrap();
    // SAFETY: this owned finite fixture changes only its own termination disposition.
    assert_ne!(
        unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) },
        libc::SIG_ERR
    );
    eprintln!("VPT_PHASE history-budget-stall");
    println!("VPT_READY");
    io::stdout().flush().unwrap();
    while monotonic().as_nanos() < fallback {
        std::thread::sleep(Duration::from_millis(1));
    }
    std::process::exit(23);
}
