#[path = "harness/deadline.rs"]
mod deadline;
#[path = "harness/identity.rs"]
mod identity;
mod support;

use deadline::{ensure_group_gone, observe_short_owned_child, panic_message, sandboxed_shell};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use support::{Sandbox, broken_output, run, stderr, stdout};

#[test]
fn late_completion_observed_after_deadline_is_rejected() {
    deadline::late_completion_observed_after_deadline_is_rejected();
}

#[test]
fn completion_observed_during_poll_after_deadline_is_rejected() {
    let sandbox = Sandbox::new("completion-during-poll");
    let mut command = sandboxed_shell(&sandbox, "exit 0");
    let mut child = support::spawn(&mut command);
    observe_short_owned_child(&mut child);
    let started = Instant::now() - support::WATCHDOG.saturating_sub(Duration::from_millis(100));
    let mut drove = false;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        support::collect_with(child, started, None, || {
            drove = true;
            while started.elapsed() <= support::WATCHDOG {
                std::thread::yield_now();
            }
        })
    }));
    assert!(drove, "the poll did not start before the deadline");
    let failure = outcome.expect_err("completion observed after the poll deadline must fail");
    let message = panic_message(&failure);
    assert!(
        message.starts_with(&format!(
            "command exceeded {} ms deadline:",
            support::WATCHDOG.as_millis()
        )),
        "{message}"
    );
}

#[test]
fn watchdog_timeout_reports_bounded_capture_and_phase() {
    let sandbox = Sandbox::new("watchdog-diagnostic");
    let stdout_payload = "A".repeat(80);
    let stderr_payload = "B".repeat(80);
    let mut command = sandboxed_shell(
        &sandbox,
        "printf '\\033%s' \"$1\"; printf '\\002%s' \"$2\" >&2; IFS= read -r answer",
    );
    command.args(["watchdog-diagnostic", &stdout_payload, &stderr_payload]);
    let started = Instant::now();
    let child = support::spawn(&mut command);
    let spawn_return = Instant::now();
    let pid = i32::try_from(child.id()).expect("owned pid");
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        support::collect(child, started, Some(spawn_return))
    }))
    .expect_err("stdin-blocked owned child must time out");
    ensure_group_gone(pid);
    let message = panic_message(&failure);
    assert!(
        message.starts_with(&format!(
            "command exceeded {} ms deadline:",
            support::WATCHDOG.as_millis()
        )),
        "{message}"
    );
    assert!(message.contains("stdout_bytes=81"), "{message}");
    assert!(message.contains("stderr_bytes=81"), "{message}");
    assert!(!message.contains("spawn_return=none"), "{message}");
    assert!(message.contains("stdout_preview=\\x1b"), "{message}");
    assert!(message.contains("stderr_preview=\\x02"), "{message}");
    assert!(
        !message.contains('\u{1b}') && !message.contains('\u{2}'),
        "{message}"
    );
    assert!(!message.contains(&"A".repeat(64)), "{message}");
    assert!(!message.contains(&"B".repeat(64)), "{message}");
    for field in [
        "spawn_return=",
        "driver=",
        "stdout_drain=",
        "stderr_drain=",
        "try_wait=",
        "child_exit=",
        "stdout_eof=",
        "stderr_eof=",
        "deadline=",
        "cleanup=",
        "poll_gap=",
        "active_step=",
        "child=",
        "status=",
    ] {
        assert!(message.contains(field), "missing {field}: {message}");
    }
}

#[test]
fn watchdog_absent_pipe_is_not_reported_as_eof() {
    deadline::watchdog_absent_pipe_is_not_reported_as_eof();
}

#[test]
fn retained_output_after_exit_obeys_the_deadline() {
    retained_peer_obeys_deadline(false);
}

#[test]
fn retained_error_after_exit_obeys_the_deadline() {
    retained_peer_obeys_deadline(true);
}

fn retained_peer_obeys_deadline(on_stderr: bool) {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    let sandbox = Sandbox::new("retained-peer");
    let mut command = sandbox.vpt();
    command.arg("--version");
    let mut child = support::spawn(&mut command);
    let (reader, peer) = UnixStream::pair().expect("owned output");
    if on_stderr {
        child.stderr = Some(OwnedFd::from(reader).into());
    } else {
        child.stdout = Some(OwnedFd::from(reader).into());
    }
    let (send, receive) = std::sync::mpsc::channel();
    let started = Instant::now();
    let worker = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            support::collect(child, started, None)
        }));
        send.send(outcome.is_err()).expect("result receiver");
    });
    let bounded =
        receive.recv_timeout(support::WATCHDOG.saturating_add(Duration::from_millis(200)));
    if bounded.is_err() {
        peer.shutdown(std::net::Shutdown::Both)
            .expect("release owned peer");
        receive
            .recv_timeout(Duration::from_millis(150))
            .expect("released collector");
    }
    while !worker.is_finished() && started.elapsed() < support::CLEANUP_BOUND {
        std::thread::yield_now();
    }
    assert!(
        worker.is_finished(),
        "collector did not finish after peer release"
    );
    worker.join().expect("collector worker");
    assert_eq!(
        bounded.ok(),
        Some(true),
        "retained output escaped its deadline"
    );
    assert!(started.elapsed() < support::CLEANUP_BOUND);
}

#[test]
fn broken_output_rejects_writes_while_a_peer_copy_survives() {
    let sandbox = Sandbox::new("broken-output-retained-peer");
    let (descriptor, reader) = broken_output();
    let retained_peer = reader.try_clone().expect("retained peer");
    drop(reader);
    let output = run(sandbox
        .vpt()
        .args(["transcode", "--json"])
        .stderr(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    drop(retained_peer);
}

#[test]
fn watchdog_terminates_the_hanging_group_and_reaps_its_child() {
    let sandbox = Sandbox::new("watchdog");
    let environment = sandbox.vpt();
    let mut command = Command::new("/bin/sh");
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(
            environment
                .get_envs()
                .map(|(key, value)| (key, value.expect("set"))),
        );
    command.args([
        "-c",
        "trap 'wait; exit 0' TERM; /bin/sleep 30 & printf '%s %s' \"$$\" \"$!\" > \"$1\"; wait",
        "watchdog",
    ]);
    command.arg(sandbox.path().join("pids"));
    let started = Instant::now();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_now(&mut command)))
        .expect_err("hanging command must fail");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .expect("panic message");
    assert!(message.contains("exceeded"), "{message}");
    assert!(started.elapsed() < support::CLEANUP_BOUND);
    let pids = std::fs::read_to_string(sandbox.path().join("pids")).expect("spawned process ids");
    for pid in pids
        .split_whitespace()
        .map(|pid| pid.parse::<i32>().expect("pid"))
    {
        // SAFETY: signal zero sends nothing; it only probes the pids recorded in this sandbox.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "process {pid} survived");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
}

#[test]
fn runner_captures_output_and_exit_status() {
    let sandbox = Sandbox::new("runner-output");
    let output = run(sandbox.vpt().arg("--version"));
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).starts_with("vpt "));
    assert_eq!(stderr(&output), "");
}

#[test]
fn watchdog_kills_a_direct_child_that_ignores_term() {
    let sandbox = Sandbox::new("ignore-term");
    let environment = sandbox.vpt();
    let mut command = Command::new("/bin/sh");
    command
        .env_clear()
        .envs(environment.get_envs().map(|(k, v)| (k, v.expect("set"))))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args([
            "-c",
            "trap '' TERM; printf '%s' \"$$\" > \"$1\"; read answer",
            "ignore-term",
        ])
        .arg(sandbox.path().join("pid"));
    let started = Instant::now();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_now(&mut command))).is_err()
    );
    let pid = std::fs::read_to_string(sandbox.path().join("pid"))
        .expect("ready pid")
        .parse::<i32>()
        .expect("pid");
    // SAFETY: signal zero sends nothing; it only probes the group recorded by this fixture.
    assert_eq!(unsafe { libc::killpg(pid, 0) }, -1, "owned group survived");
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    assert!(started.elapsed() < support::CLEANUP_BOUND);
}

#[cfg(feature = "dev-tools")]
#[test]
fn watchdog_parent_reaps_its_same_group_descendant() {
    let sandbox = Sandbox::new("descendant-reaping");
    let environment = sandbox.vpt();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vpt-fake-engine"));
    command
        .env("UNOWNED_SENTINEL", "present")
        .env_clear()
        .envs(environment.get_envs().map(|(k, v)| (k, v.expect("set"))))
        .arg("reap-descendant")
        .arg(sandbox.path().join("ready"))
        .arg(sandbox.path().join("reaped"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let started = Instant::now();
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_now(&mut command)));
    assert!(failure.is_err(), "retained child output must time out");
    let ready = std::fs::read_to_string(sandbox.path().join("ready")).expect("ready record");
    let mut pids = ready.split_whitespace();
    let group = pids.next().expect("group pid").parse::<i32>().expect("pid");
    let child = pids.next().expect("child pid").parse::<i32>().expect("pid");
    assert_eq!(pids.next(), Some("absent"), "fixture inherited unowned env");
    assert_eq!(
        pids.next(),
        Some(sandbox.path().join("home").to_str().expect("home path"))
    );
    assert_eq!(
        pids.next(),
        Some(sandbox.path().join("bin").to_str().expect("PATH"))
    );
    let reaped = std::fs::read_to_string(sandbox.path().join("reaped")).expect("reaping record");
    assert_eq!(reaped.trim(), format!("{child} 15"));
    assert_eq!(
        // SAFETY: signal zero sends nothing; it only probes the group this fixture recorded.
        unsafe { libc::killpg(group, 0) },
        -1,
        "owned group survived"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    assert!(started.elapsed() < support::CLEANUP_BOUND);
}

fn run_now(command: &mut Command) -> std::process::Output {
    let started = Instant::now();
    let child = support::spawn(command);
    support::collect(child, started, Some(Instant::now()))
}
