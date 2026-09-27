use crate::support::{self, Sandbox};
use std::process::{Child, Command, Stdio};
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
    reap_short_owned_child(&mut child);
    let started = Instant::now() - Duration::from_millis(501);
    let mut drove = false;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        support::collect_with(child, started, None, || drove = true)
    }));
    assert!(!drove, "expired command drove the terminal");
    let failure = outcome.expect_err("completion observed after deadline must fail");
    let message = panic_message(&failure);
    assert!(
        message.starts_with("command exceeded 500 ms deadline:"),
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
    let failure = std::panic::catch_unwind(|| support::collect(child, started, Some(spawn_return)))
        .expect_err("stdin-blocked owned child must time out");
    ensure_group_gone(pid);
    let message = panic_message(&failure);
    assert!(
        message.starts_with("command exceeded 500 ms deadline:"),
        "{message}"
    );
    assert!(message.contains("stdout_eof=none"), "{message}");
    assert!(message.contains("stdout_closed=true"), "{message}");
    assert!(message.contains("stderr_eof=none"), "{message}");
    assert!(message.contains("stderr_closed=false"), "{message}");
    assert!(message.contains("stderr_bytes=5"), "{message}");
}

pub(super) fn reap_short_owned_child(child: &mut Child) {
    let ready_by = Instant::now() + Duration::from_millis(400);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < ready_by => std::thread::yield_now(),
            result => {
                let pid = i32::try_from(child.id()).expect("owned pid");
                // SAFETY: spawn created this child's owned process group.
                unsafe { libc::killpg(pid, libc::SIGKILL) };
                child.wait().expect("reap owned child after setup failure");
                panic!("short owned child did not exit: {result:?}");
            }
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
    // SAFETY: this PID is from the owned direct child spawned by this test.
    let group = unsafe { libc::killpg(pid, 0) };
    if group == 0 {
        // SAFETY: the same owned process group remains after the collector returned.
        unsafe { libc::killpg(pid, libc::SIGKILL) };
        let until = Instant::now() + Duration::from_millis(100);
        let mut status = 0;
        while unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) } == 0
            && Instant::now() < until
        {
            std::thread::yield_now();
        }
        panic!("owned command group survived collector cleanup: {pid}");
    }
    assert_eq!(group, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH),
        "owned group probe failed unexpectedly"
    );
    let mut status = 0;
    // SAFETY: only the owned direct child may match this exact PID.
    let reaped = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
    assert_eq!(reaped, -1, "collector left its owned child unreaped");
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD),
        "owned child probe failed unexpectedly"
    );
}
