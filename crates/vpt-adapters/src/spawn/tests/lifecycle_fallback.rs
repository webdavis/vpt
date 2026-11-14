use super::{fixture_child::FixtureChild, lifecycle_fixture::Fixture, watchdog::WATCHDOG};
use std::io;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::Stdio;
use std::time::Instant;

#[test]
fn pending_child_fallback_records_its_owned_marker_and_exit() {
    let fixture = Fixture::new();
    let status = observe_fallback(&fixture);
    assert_eq!(status.code(), Some(75), "child-local fallback exit");
    assert_eq!(
        std::fs::read(fixture.root.path().join("fallback")).expect("owned fallback marker"),
        b"expired"
    );
}

#[test]
fn pending_child_fallback_write_failure_has_a_distinct_exit() {
    let fixture = Fixture::new();
    let marker = fixture.root.path().join("fallback");
    std::fs::create_dir(&marker).expect("owned marker obstruction");
    let status = observe_fallback(&fixture);
    assert_eq!(status.code(), Some(76), "fallback evidence failure exit");
    assert!(marker.is_dir(), "owned obstruction preserved");
}

fn observe_fallback(fixture: &Fixture) -> std::process::ExitStatus {
    let mut command = fixture.pending();
    command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let started = Instant::now();
    let child = command.spawn().expect("owned finite-fallback fixture");
    let mut child = FixtureChild::new(child);
    loop {
        match child.terminal() {
            Ok(true) => return finish(&mut child, started, false),
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => panic!("fallback observation: {:?}", error.kind()),
        }
        if started.elapsed() >= WATCHDOG {
            let _ = child.kill();
            return finish(&mut child, started, true);
        }
        std::thread::yield_now();
    }
}

fn finish(
    child: &mut FixtureChild,
    started: Instant,
    parent_bound: bool,
) -> std::process::ExitStatus {
    let pid = child.pid();
    let status = child.wait().expect("reap exact fallback fixture");
    // SAFETY: this initialized observation cannot consume or reacquire the recorded child.
    let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
    assert_eq!(
        // SAFETY: WNOWAIT is observational after the completed exact-child wait.
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
    // SAFETY: signal zero observes only the recorded group and never restores authority.
    assert_eq!(unsafe { libc::killpg(pid, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    eprintln!(
        "fallback-control code={:?} signal={:?} parent_elapsed_ms={} parent_bound={parent_bound}",
        status.code(),
        status.signal(),
        started.elapsed().as_millis()
    );
    status
}
