use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const REAPER_BOUND: Duration = Duration::from_millis(850);

pub(super) fn fixture_command(root: &Path, role: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args([
            "--exact",
            &format!("spawn::tests::reaper::{role}"),
            "--nocapture",
        ])
        .env_clear()
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_STATE_HOME", root.join("state"))
        .env("VPT_CONFIG", root.join("config.toml"))
        .env("PATH", root.join("bin"))
        .env("VPT_PROCESS_FIXTURE", root);
    command
}

#[test]
fn cooperative_reaper() {
    let Some(root) = std::env::var_os("VPT_PROCESS_FIXTURE") else {
        return;
    };
    let root = Path::new(&root);
    let started = Instant::now();
    let mut control = std::io::stdin();
    nonblocking(&control);
    let group = read_group(&mut control, started);
    let mut command = fixture_command(root, "group_participant");
    command
        .env(
            "VPT_PROCESS_CLOSED_PIPES",
            std::env::var("VPT_PROCESS_CLOSED_PIPES").unwrap_or_default(),
        )
        .process_group(group)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let child = command.spawn().expect("owned group participant");
    let mut participant = FixtureChild::new(child);
    let pid = participant.pid();
    // SAFETY: the reaper's stdout/stderr copies are no longer used after child inheritance.
    unsafe {
        libc::close(libc::STDOUT_FILENO);
        libc::close(libc::STDERR_FILENO);
    }
    let (status, fallback) = loop {
        match participant.terminal() {
            Ok(true) => {
                break (
                    participant.wait().expect("reap terminal participant"),
                    false,
                );
            }
            Ok(false) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => panic!("participant observation: {:?}", error.kind()),
        }
        let mut cancellation = [0];
        let cancelled = match control.read(&mut cancellation) {
            Ok(_) => true,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) =>
            {
                false
            }
            Err(_) => true,
        };
        if cancelled || started.elapsed() >= REAPER_BOUND {
            let _ = participant.kill();
            break (participant.wait().expect("reap owned participant"), true);
        }
        std::thread::yield_now();
    };
    std::fs::write(
        root.join("reaped"),
        format!("{pid} {} {fallback}", status.signal().unwrap_or(0)),
    )
    .expect("reaping record");
    drop(participant);
    std::process::exit(0);
}

fn read_group(control: &mut impl Read, started: Instant) -> libc::pid_t {
    let mut input = Vec::new();
    let mut byte = [0];
    loop {
        assert!(
            started.elapsed() < REAPER_BOUND,
            "group registration timed out"
        );
        match control.read(&mut byte) {
            Ok(0) => panic!("group registration cancelled"),
            Ok(_) if byte[0] == b'\n' => {
                let group = std::str::from_utf8(&input)
                    .expect("group ASCII")
                    .parse::<libc::pid_t>()
                    .expect("group pid");
                assert!(group > 1, "owned group");
                return group;
            }
            Ok(_) => {
                assert!(input.len() < 16, "bounded group record");
                input.push(byte[0]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) =>
            {
                std::thread::yield_now()
            }
            Err(error) => panic!("read group control: {:?}", error.kind()),
        }
    }
}

fn nonblocking(file: &impl AsRawFd) {
    // SAFETY: the live fixture descriptor is borrowed; only its status flags change.
    let result = unsafe {
        let flags = libc::fcntl(file.as_raw_fd(), libc::F_GETFL);
        assert_ne!(flags, -1, "fixture descriptor flags");
        libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK)
    };
    assert_ne!(result, -1, "nonblocking fixture control");
}

#[test]
fn group_participant() {
    let Some(root) = std::env::var_os("VPT_PROCESS_FIXTURE") else {
        return;
    };
    // SAFETY: the post-exec fixture changes only its own TERM disposition.
    unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
    if std::env::var("VPT_PROCESS_CLOSED_PIPES").as_deref() == Ok("1") {
        // SAFETY: this fixture no longer uses its standard descriptors.
        unsafe {
            libc::close(libc::STDIN_FILENO);
            libc::close(libc::STDOUT_FILENO);
            libc::close(libc::STDERR_FILENO);
        }
    } else {
        std::io::stdout()
            .write_all(b"{}")
            .expect("owned retained output");
    }
    super::lifecycle_fixture::publish_ready(
        Path::new(&root),
        std::process::id().to_string().as_bytes(),
    )
    .expect("participant readiness");
    loop {
        std::thread::park();
    }
}
use super::fixture_child::FixtureChild;
