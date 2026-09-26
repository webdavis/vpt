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
    let pid = participant.group.child.id();
    participant.ready().unwrap();
    let second = participant.ready();
    drop(participant);
    assert_reaped(pid, started + Duration::from_millis(900));
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
    let first_pid = first.group.child.id();
    first.ready().unwrap();
    first.send("continue").unwrap();
    assert!(first.finish().unwrap().status.success());
    assert_reaped(first_pid, started + Duration::from_millis(900));

    let mut later = child_for_root(
        "supervision_stall",
        temp.path(),
        std::process::id(),
        started,
    )
    .unwrap();
    let later_pid = later.group.child.id();
    later.ready().unwrap();
    later.send("continue").unwrap();
    let error = later.finish().unwrap_err();
    assert_reaped(later_pid, started + Duration::from_millis(900));
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
    let pid = participant.group.child.id();
    participant.ready().unwrap();
    participant.send("continue").unwrap();
    let error = participant.finish().unwrap_err();
    assert_reaped(pid, started + Duration::from_millis(900));
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("phase=pre-sql-stall"));
    assert!(started.elapsed() < Duration::from_secs(1));
}

fn assert_reaped(pid: u32, deadline: Instant) {
    let pid = i32::try_from(pid).unwrap();
    let mut status = 0;
    // SAFETY: this exact child was spawned by the root fixture and owns its group.
    let observed = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
    let error = io::Error::last_os_error();
    if observed == 0 {
        // SAFETY: terminate only the still-owned child group before reporting a cleanup failure.
        unsafe { libc::killpg(pid, libc::SIGKILL) };
        loop {
            // SAFETY: the root fixture remains this child's parent and reaping authority.
            let reaped = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
            if reaped == pid
                || (reaped == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD))
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "owned child cleanup backstop expired"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    assert_eq!(
        observed, -1,
        "watchdog did not reap child {pid}: status={status}"
    );
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    // SAFETY: signal zero only observes this fixture-owned process group.
    assert_eq!(unsafe { libc::killpg(pid, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}
