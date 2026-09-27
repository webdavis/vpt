#[path = "identity/status.rs"]
mod status;

use crate::support::{self, Sandbox};
use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct SignalRecord {
    signal: i32,
    terminal: Result<Option<(i32, i32)>, i32>,
    released: io::Result<()>,
    exit_observed: bool,
}

fn terminal(pid: i32) -> Result<Option<(i32, i32)>, i32> {
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

fn command(sandbox: &Sandbox, scenario: &str) -> Command {
    let environment = sandbox.vpt();
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args(["--exact", "identity::finite_child", "--nocapture"])
        .env_clear()
        .envs(
            environment
                .get_envs()
                .map(|(key, value)| (key, value.expect("set"))),
        )
        .env("VPT_IDENTITY_SCENARIO", scenario)
        .env("VPT_IDENTITY_ROOT", sandbox.path())
        .env("VPT_IDENTITY_PARENT", std::process::id().to_string())
        .current_dir(sandbox.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
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
fn collection_retains_exited_identity_through_the_last_signal() {
    let sandbox = Sandbox::new("identity-exit");
    let started = Instant::now();
    let (record, receive) = recorder(None, started + Duration::from_millis(500));
    let child = support::spawn_with_signal(&mut command(&sandbox, "exit"), started, record);
    let pid = i32::try_from(child.id()).expect("owned child pid");
    let output = support::collect_with(child, started, None, || {});
    assert_eq!(output.status.code(), Some(23));
    assert_eq!(output.status.signal(), None);
    assert!(!output.status.core_dumped());
    assert!(String::from_utf8_lossy(&output.stdout).contains("identity-output"));
    assert_identity(receive, pid, (libc::CLD_EXITED, 23));
}

#[test]
fn collection_preserves_signal_status_while_retaining_identity() {
    let sandbox = Sandbox::new("identity-signal");
    let started = Instant::now();
    let (record, receive) = recorder(None, started + Duration::from_millis(500));
    let child = support::spawn_with_signal(&mut command(&sandbox, "signal"), started, record);
    let pid = i32::try_from(child.id()).expect("owned child pid");
    let output = support::collect_with(child, started, None, || {});
    assert_eq!(output.status.code(), None);
    assert_eq!(output.status.signal(), Some(libc::SIGTERM));
    assert!(!output.status.core_dumped());
    assert_identity(receive, pid, (libc::CLD_KILLED, libc::SIGTERM));
}

#[test]
fn grace_observation_retains_identity_before_the_last_signal() {
    let sandbox = Sandbox::new("identity-grace");
    let started = Instant::now();
    let (record, receive) = recorder(
        Some(sandbox.path().join("identity-release")),
        started + Duration::from_millis(500),
    );
    let group = support::spawn_with_signal(&mut command(&sandbox, "grace"), started, record);
    let pid = i32::try_from(group.id()).expect("owned child pid");
    let ready = wait_until(started + Duration::from_millis(500), || {
        sandbox.path().join("identity-ready").is_file()
    });
    drop(group);
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

#[test]
fn driver_failure_reaps_the_finite_child_before_unwinding() {
    let sandbox = Sandbox::new("identity-driver-failure");
    let started = Instant::now();
    let (record, receive) = recorder(
        Some(sandbox.path().join("identity-release")),
        started + Duration::from_millis(500),
    );
    let child = support::spawn_with_signal(&mut command(&sandbox, "grace"), started, record);
    let pid = i32::try_from(child.id()).expect("owned child pid");
    let mut ready = false;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        support::collect_with(child, started, None, || {
            ready = wait_until(started + Duration::from_millis(500), || {
                sandbox.path().join("identity-ready").is_file()
            });
            panic!("identity driver failure");
        })
    }));
    let records: Vec<_> = receive.try_iter().collect();
    assert_eq!(terminal(pid), Err(libc::ECHILD), "unwinding did not reap");
    assert!(ready, "finite child did not enter its controlled wait");
    let failure = outcome.expect_err("driver must fail");
    assert_eq!(
        super::deadline::panic_message(&failure),
        "identity driver failure"
    );
    assert_eq!(records.len(), 2, "{records:?}");
    assert_eq!(records[0].signal, libc::SIGTERM);
    assert_eq!(records[0].terminal, Ok(None), "leader exited before grace");
    assert!(records[0].released.is_ok(), "fixture release: {records:?}");
    assert!(
        records[0].exit_observed,
        "fixture did not exit: {records:?}"
    );
    assert_eq!(records[1].signal, libc::SIGKILL);
    assert_eq!(records[1].terminal, Ok(Some((libc::CLD_EXITED, 23))));
}

#[test]
fn finite_child() {
    let Ok(scenario) = std::env::var("VPT_IDENTITY_SCENARIO") else {
        return;
    };
    let expected = std::env::var("VPT_IDENTITY_PARENT").expect("fixture parent");
    // SAFETY: getppid observes this fixture's direct parent.
    assert_eq!(unsafe { libc::getppid() }.to_string(), expected);
    println!("identity-output");
    io::stdout().flush().unwrap();
    if scenario == "grace" {
        let root = PathBuf::from(std::env::var_os("VPT_IDENTITY_ROOT").expect("fixture root"));
        std::fs::write(root.join("identity-ready"), b"ready").unwrap();
        let until = Instant::now() + Duration::from_millis(250);
        while !root.join("identity-release").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
    } else if scenario == "signal" {
        // SAFETY: this finite fixture raises a terminating signal only on itself.
        unsafe { libc::raise(libc::SIGTERM) };
        std::process::exit(99);
    }
    std::process::exit(23);
}
