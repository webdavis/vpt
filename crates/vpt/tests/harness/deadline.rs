use crate::support::{self, ProcessGroup, Sandbox};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn sandboxed_shell(sandbox: &Sandbox, script: &str) -> Command {
    let environment = sandbox.vpt();
    let mut command = Command::new("/bin/sh");
    command
        .env_clear()
        .envs(
            environment
                .get_envs()
                .map(|(key, value)| (key, value.expect("set"))),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(["-c", script]);
    command
}

pub(super) fn late_completion_observed_after_deadline_is_rejected() {
    let sandbox = Sandbox::new("late-completion");
    let mut command = sandboxed_shell(&sandbox, "exit 0");
    let mut child = support::spawn(&mut command);
    observe_short_owned_child(&mut child);
    let started = Instant::now() - support::WATCHDOG.saturating_add(Duration::from_millis(1));
    let mut drove = false;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        support::collect_with(child, started, None, || drove = true)
    }));
    assert!(!drove, "expired command drove the terminal");
    let failure = outcome.expect_err("completion observed after deadline must fail");
    let message = panic_message(&failure);
    assert!(
        message.starts_with(&format!(
            "command exceeded {} ms deadline:",
            support::WATCHDOG.as_millis()
        )),
        "{message}"
    );
}

pub(super) fn watchdog_absent_pipe_is_not_reported_as_eof() {
    let sandbox = Sandbox::new("watchdog-absent-pipe");
    let mut command = sandboxed_shell(&sandbox, "printf 'ready' >&2; IFS= read -r answer");
    command.stdout(Stdio::null());
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
    assert!(message.contains("stdout_eof=none"), "{message}");
    assert!(message.contains("stdout_closed=true"), "{message}");
    assert!(message.contains("stderr_eof=none"), "{message}");
    assert!(message.contains("stderr_closed=false"), "{message}");
    assert!(message.contains("stderr_bytes=5"), "{message}");
}

pub(super) fn observe_short_owned_child(child: &mut ProcessGroup) {
    let ready_by = Instant::now() + support::WATCHDOG;
    loop {
        match child.observe() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < ready_by => std::thread::yield_now(),
            result => panic!("short owned child did not exit: {result:?}"),
        }
    }
}

pub(super) fn panic_message(failure: &Box<dyn std::any::Any + Send>) -> &str {
    failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .expect("panic message")
}

pub(super) fn ensure_group_gone(pid: i32) {
    // SAFETY: signal zero is an observational probe of the recorded group number.
    let group = unsafe { libc::killpg(pid, 0) };
    let group_error = std::io::Error::last_os_error();
    assert_eq!(group, -1, "recorded group number remains present: {pid}");
    assert_eq!(
        group_error.raw_os_error(),
        Some(libc::ESRCH),
        "owned group probe failed unexpectedly"
    );
    // SAFETY: siginfo_t is a plain C struct for which all-zero bytes are valid.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // SAFETY: initialized storage receives a non-consuming numeric PID observation.
    let observed = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    let child_error = std::io::Error::last_os_error();
    assert_eq!(observed, -1, "recorded child number remains waitable");
    assert_eq!(
        child_error.raw_os_error(),
        Some(libc::ECHILD),
        "owned child probe failed unexpectedly"
    );
}
