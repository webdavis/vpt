mod support;

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use support::{Sandbox, broken_output, run, stderr, stdout};

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
        let outcome = std::panic::catch_unwind(|| support::collect(child, started));
        send.send(outcome.is_err()).expect("result receiver");
    });
    let bounded = receive.recv_timeout(Duration::from_millis(700));
    if bounded.is_err() {
        peer.shutdown(std::net::Shutdown::Both)
            .expect("release owned peer");
        receive
            .recv_timeout(Duration::from_millis(150))
            .expect("released collector");
    }
    while !worker.is_finished() && started.elapsed() < Duration::from_millis(900) {
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
    assert!(started.elapsed() < Duration::from_secs(1));
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
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&mut command)))
        .expect_err("hanging command must fail");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .expect("panic message");
    assert!(message.contains("exceeded"), "{message}");
    assert!(started.elapsed() < Duration::from_secs(1));
    let pids = std::fs::read_to_string(sandbox.path().join("pids")).expect("spawned process ids");
    for pid in pids
        .split_whitespace()
        .map(|pid| pid.parse::<i32>().expect("pid"))
    {
        // Signal zero checks only the fixture processes recorded in this sandbox.
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
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&mut command))).is_err());
    let pid = std::fs::read_to_string(sandbox.path().join("pid"))
        .expect("ready pid")
        .parse::<i32>()
        .expect("pid");
    assert_eq!(unsafe { libc::killpg(pid, 0) }, -1, "owned group survived");
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    assert!(started.elapsed() < Duration::from_secs(1));
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
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&mut command)));
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
    // Signal zero checks only the group recorded by this owned fixture.
    assert_eq!(
        unsafe { libc::killpg(group, 0) },
        -1,
        "owned group survived"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    assert!(started.elapsed() < Duration::from_secs(1));
}
