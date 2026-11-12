mod status;

use super::{ChildGuard, spawn_with_signal};
use crate::database::tests::support::root;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct SignalRecord {
    signal: i32,
    terminal: Result<Option<(i32, i32)>, i32>,
    released: io::Result<()>,
    exit_observed: bool,
}

pub(super) fn terminal(pid: i32) -> Result<Option<(i32, i32)>, i32> {
    // SAFETY: initialized storage receives a non-consuming observation of this exact child.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result == -1 {
        return Err(io::Error::last_os_error().raw_os_error().unwrap_or(-1));
    }
    if info.si_pid == 0 {
        Ok(None)
    } else if info.si_pid == pid {
        Ok(Some((info.si_code, info.si_status)))
    } else {
        Err(libc::EINVAL)
    }
}

fn recorder(
    release: Option<PathBuf>,
    deadline: Instant,
) -> (
    impl FnMut(i32, i32) + Send + 'static,
    Receiver<SignalRecord>,
) {
    let (send, receive) = mpsc::channel();
    let record = move |pid, signal| {
        let observed = terminal(pid);
        let released = if signal == libc::SIGTERM {
            release
                .as_ref()
                .map_or(Ok(()), |path| std::fs::write(path, b"continue"))
        } else {
            Ok(())
        };
        let exit_observed = signal != libc::SIGTERM
            || release.is_none()
            || wait_until(deadline, || matches!(terminal(pid), Ok(Some(_))));
        let _ = send.send(SignalRecord {
            signal,
            terminal: observed,
            released,
            exit_observed,
        });
    };
    (record, receive)
}

fn wait_until(deadline: Instant, mut ready: impl FnMut() -> bool) -> bool {
    loop {
        if ready() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn participant(scenario: &str, directory: &Path) -> (ChildGuard, Receiver<SignalRecord>, i32) {
    let release = (scenario == "identity_grace").then(|| directory.join("identity-release"));
    let started = Instant::now();
    let (record, receive) = recorder(release, started + super::WATCHDOG);
    let child = spawn_with_signal(
        scenario,
        directory,
        Some(std::process::id().to_string().into()),
        started,
        record,
    )
    .expect("finite identity participant");
    let pid = i32::try_from(child.group.id()).expect("owned child pid");
    (child, receive, pid)
}

fn assert_identity(receive: Receiver<SignalRecord>, pid: i32, expected: (i32, i32)) {
    let records: Vec<_> = receive.try_iter().collect();
    assert_eq!(
        terminal(pid),
        Err(libc::ECHILD),
        "final cleanup did not reap"
    );
    assert_eq!(
        records.iter().map(|event| event.signal).collect::<Vec<_>>(),
        [libc::SIGTERM, libc::SIGKILL],
        "both cleanup signal requests must be observed"
    );
    for event in records {
        assert!(event.released.is_ok(), "fixture release: {event:?}");
        assert!(event.exit_observed, "fixture did not exit: {event:?}");
        assert_eq!(
            event.terminal,
            Ok(Some(expected)),
            "leader identity was released before signal {}",
            event.signal
        );
    }
}

#[test]
fn finish_retains_exited_identity_through_the_last_signal() {
    let (directory, _root) = root();
    let (child, receive, pid) = participant("identity_exit", directory.path());
    let output = child.finish();
    let output = output.expect("finite child output");
    assert_eq!(output.status.code(), Some(23));
    assert_eq!(output.status.signal(), None);
    assert!(!output.status.core_dumped());
    assert_identity(receive, pid, (libc::CLD_EXITED, 23));
}

#[test]
fn finish_preserves_signal_status_while_retaining_identity() {
    let (directory, _root) = root();
    let (child, receive, pid) = participant("identity_signal", directory.path());
    let output = child.finish();
    let output = output.expect("signaled child output");
    assert_eq!(output.status.code(), None);
    assert_eq!(output.status.signal(), Some(libc::SIGTERM));
    assert!(!output.status.core_dumped());
    assert_identity(receive, pid, (libc::CLD_KILLED, libc::SIGTERM));
}

#[test]
fn readiness_failure_retains_identity_until_cleanup() {
    let (directory, _root) = root();
    let (mut child, receive, pid) = participant("identity_exit", directory.path());
    let ready = child.ready();
    drop(child);
    let error = ready.expect_err("finite child never announces readiness");
    assert!(error.to_string().starts_with("child exited before ready:"));
    assert_identity(receive, pid, (libc::CLD_EXITED, 23));
}

#[test]
fn timeout_diagnostic_retains_exited_identity_until_cleanup() {
    let (directory, _root) = root();
    let (mut child, receive, pid) = participant("identity_exit", directory.path());
    let exited = wait_until(child.deadline, || matches!(terminal(pid), Ok(Some(_))));
    let error = child.timeout();
    drop(child);
    assert!(
        exited,
        "finite child did not exit before diagnostic observation"
    );
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(error.to_string().contains("child=exited:"));
    assert_identity(receive, pid, (libc::CLD_EXITED, 23));
}

#[test]
fn grace_observation_retains_identity_before_the_last_signal() {
    let (directory, _root) = root();
    let (child, receive, pid) = participant("identity_grace", directory.path());
    let ready = wait_until(child.deadline, || {
        directory.path().join("identity-ready").is_file()
    });
    drop(child);
    let records: Vec<_> = receive.try_iter().collect();
    assert_eq!(
        terminal(pid),
        Err(libc::ECHILD),
        "final cleanup did not reap"
    );
    assert!(ready, "finite child did not enter its controlled wait");
    assert_eq!(records.len(), 2, "{records:?}");
    assert_eq!(records[0].signal, libc::SIGTERM);
    assert_eq!(records[0].terminal, Ok(None), "leader exited before grace");
    assert!(records[0].released.is_ok(), "fixture release: {records:?}");
    assert!(
        records[0].exit_observed,
        "fixture did not exit: {records:?}"
    );
    assert_eq!(records[1].signal, libc::SIGKILL);
    assert_eq!(
        records[1].terminal,
        Ok(Some((libc::CLD_EXITED, 23))),
        "grace released the leader identity before KILL"
    );
}

pub(super) fn finite_child(scenario: &str, directory: &Path) -> ! {
    if scenario == "identity_grace" {
        std::fs::write(directory.join("identity-ready"), b"ready").unwrap();
        let until = Instant::now() + super::WATCHDOG;
        while !directory.join("identity-release").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
    } else if scenario == "identity_signal" {
        // SAFETY: this finite fixture raises a terminating signal only on itself.
        unsafe { libc::raise(libc::SIGTERM) };
        std::process::exit(99);
    }
    std::process::exit(23);
}
