mod support;

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use support::{Sandbox, broken_output, run, stderr, stdout};

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
