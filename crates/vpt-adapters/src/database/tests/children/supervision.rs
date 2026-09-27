use super::child_for_root;
use crate::database::tests::support::root;
use std::io;
use std::time::{Duration, Instant};

#[test]
fn a_second_milestone_cannot_reuse_the_first_ready_message() {
    let (temp, _root) = root();
    let started = Instant::now();
    let mut participant = child_for_root(
        "supervision_twice",
        temp.path(),
        std::process::id(),
        started,
    )
    .unwrap();
    let pid = participant.group.id();
    participant.ready().unwrap();
    let second = participant.ready();
    drop(participant);
    assert_reaped(pid);
    assert!(
        matches!(second, Err(error) if error.kind() == io::ErrorKind::TimedOut),
        "the participant is still waiting at its first milestone"
    );
}

#[test]
fn a_later_peer_keeps_the_original_scenario_budget() {
    let (temp, _root) = root();
    let observed = Instant::now();
    let started = observed - Duration::from_millis(400);
    let mut first = child_for_root(
        "supervision_ready",
        temp.path(),
        std::process::id(),
        started,
    )
    .unwrap();
    let first_pid = first.group.id();
    first.ready().unwrap();
    first.send("continue").unwrap();
    assert!(first.finish().unwrap().status.success());
    assert_reaped(first_pid);

    let mut later = child_for_root(
        "supervision_stall",
        temp.path(),
        std::process::id(),
        started,
    )
    .unwrap();
    let later_pid = later.group.id();
    later.ready().unwrap();
    later.send("continue").unwrap();
    let error = later.finish().unwrap_err();
    assert_reaped(later_pid);
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("phase=pre-sql-stall"));
    assert!(
        observed.elapsed() < Duration::from_millis(300),
        "later peer reset the scenario budget: {:?}",
        observed.elapsed()
    );
}

#[test]
fn a_direct_participant_stalled_before_sql_is_terminated_and_reaped() {
    let (temp, _root) = root();
    let started = Instant::now();
    let mut participant = child_for_root(
        "supervision_stall",
        temp.path(),
        std::process::id(),
        started,
    )
    .unwrap();
    let pid = participant.group.id();
    participant.ready().unwrap();
    participant.send("continue").unwrap();
    let error = participant.finish().unwrap_err();
    assert_reaped(pid);
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("phase=pre-sql-stall"));
    assert!(started.elapsed() < Duration::from_secs(1));
    let elapsed_ms = error
        .to_string()
        .split_ascii_whitespace()
        .find_map(|part| part.strip_prefix("parent_elapsed_ms="))
        .and_then(|value| value.parse::<u128>().ok())
        .expect("timeout reports numeric parent elapsed milliseconds");
    assert!(elapsed_ms >= 500, "timeout preceded the operation deadline");
}

fn assert_reaped(pid: u32) {
    let pid = i32::try_from(pid).unwrap();
    // SAFETY: initialized storage receives a non-consuming numeric PID observation.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let observed = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    let error = io::Error::last_os_error();
    assert_eq!(
        observed, -1,
        "recorded child number remains waitable: {pid}"
    );
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    // SAFETY: signal zero is an observational probe of the recorded group number.
    assert_eq!(unsafe { libc::killpg(pid, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}
