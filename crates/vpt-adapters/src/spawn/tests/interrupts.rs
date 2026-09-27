use super::super::{install_interrupt_handlers, interrupted, production_controls};
use super::{fixture_child::FixtureChild, lifecycle_fixture::Fixture, watchdog::WATCHDOG};
use std::io;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::Stdio;
use std::time::Instant;

#[test]
fn installed_handler_records_sigint_in_an_isolated_child() {
    check_signal(libc::SIGINT);
}

#[test]
fn installed_handler_records_sigterm_in_an_isolated_child() {
    check_signal(libc::SIGTERM);
}

#[test]
fn installed_handler_records_sighup_in_an_isolated_child() {
    check_signal(libc::SIGHUP);
}

fn check_signal(signal: libc::c_int) {
    let fixture = Fixture::new();
    let exe = std::env::current_exe().expect("test executable");
    let mut command = fixture.command(
        exe.to_str().expect("fixture path"),
        &[
            "--exact",
            "spawn::tests::interrupts::isolated_signal_child",
            "--nocapture",
        ],
    );
    command
        .env("VPT_SIGNAL_FIXTURE", fixture.root.path())
        .env("VPT_SIGNAL_NUMBER", signal.to_string())
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let started = Instant::now();
    let child = command.spawn().expect("owned self-signal child");
    let mut child = FixtureChild::new(child);
    let mut parent_bound = false;
    loop {
        match child.terminal() {
            Ok(true) => break,
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => panic!("self-signal observation: {:?}", error.kind()),
        }
        if started.elapsed() >= WATCHDOG {
            let _ = child.kill();
            parent_bound = true;
            break;
        }
        std::thread::yield_now();
    }
    let pid = child.pid();
    let status = child.wait().expect("reap exact self-signal child");
    assert_released(pid);
    let handler_observed = std::fs::read(fixture.root.path().join("handler-observed"))
        .is_ok_and(|bytes| bytes == b"true");
    let callback_observed = std::fs::read(fixture.root.path().join("callback-observed"))
        .ok()
        .map(|bytes| bytes == b"true");
    eprintln!(
        "self-signal-control expected={signal} code={:?} signal={:?} parent_elapsed_ms={} parent_bound={parent_bound} handler_observed={handler_observed} callback_observed={callback_observed:?}",
        status.code(),
        status.signal(),
        started.elapsed().as_millis()
    );
    assert!(!parent_bound, "self-signal fixture reached parent bound");
    assert_eq!(
        std::fs::read_to_string(fixture.root.path().join("before-signal"))
            .expect("self-signal readiness"),
        signal.to_string()
    );
    assert!(
        status.signal().is_none() || status.signal() == Some(signal),
        "unexpected terminating signal"
    );
    assert_eq!(
        status.code(),
        Some(0),
        "installed signal handler must return"
    );
    assert_eq!(
        std::fs::read(fixture.root.path().join("observed")).expect("interrupt observation marker"),
        b"true"
    );
}

fn assert_released(pid: libc::pid_t) {
    // SAFETY: zero initialization provides valid waitid output storage.
    let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
    // SAFETY: WNOWAIT only observes the former child number after exact reaping.
    assert_eq!(
        unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        },
        -1
    );
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
    // SAFETY: signal zero only observes the former group number.
    assert_eq!(unsafe { libc::killpg(pid, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}

#[test]
fn isolated_signal_child() {
    let Some(root) = std::env::var_os("VPT_SIGNAL_FIXTURE") else {
        return;
    };
    let signal = std::env::var("VPT_SIGNAL_NUMBER")
        .expect("owned signal selection")
        .parse::<libc::c_int>()
        .expect("signal number");
    assert!(matches!(
        signal,
        libc::SIGINT | libc::SIGTERM | libc::SIGHUP
    ));
    let root = std::path::PathBuf::from(root);
    prepare_signal(signal);
    install_interrupt_handlers();
    let controls = production_controls();
    assert!(!interrupted(), "fresh child interrupt state");
    assert!(!(controls.interrupted)(), "fresh production cancellation");
    std::fs::write(root.join("before-signal"), signal.to_string()).expect("self-signal readiness");
    // SAFETY: only this isolated fixture thread receives the selected signal.
    assert_eq!(unsafe { libc::raise(signal) }, 0, "raise selected signal");
    assert!(interrupted(), "installed handler records interruption");
    std::fs::write(root.join("handler-observed"), b"true").expect("handler observation marker");
    let callback = (controls.interrupted)();
    std::fs::write(root.join("callback-observed"), callback.to_string())
        .expect("callback observation marker");
    assert!(callback, "production callback records interruption");
    std::fs::write(root.join("observed"), b"true").expect("interrupt observation marker");
}

fn prepare_signal(signal: libc::c_int) {
    // SAFETY: all-zero sigaction is valid before explicitly setting its fields.
    let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
    action.sa_sigaction = libc::SIG_DFL;
    // SAFETY: the mask is owned initialized storage.
    assert_eq!(unsafe { libc::sigemptyset(&mut action.sa_mask) }, 0);
    // SAFETY: this post-exec fixture owns its process-local signal disposition.
    assert_eq!(
        unsafe { libc::sigaction(signal, &action, std::ptr::null_mut()) },
        0
    );
    let mut selected = action.sa_mask;
    // SAFETY: selection is one validated fixture signal in owned mask storage.
    assert_eq!(unsafe { libc::sigaddset(&mut selected, signal) }, 0);
    // SAFETY: this changes only the fixture thread's selected signal mask.
    assert_eq!(
        unsafe { libc::pthread_sigmask(libc::SIG_UNBLOCK, &selected, std::ptr::null_mut()) },
        0
    );
}
