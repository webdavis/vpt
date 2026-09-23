use crate::support::{self, Sandbox};
use std::ffi::{CStr, OsStr};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::process::{Command, Output, Stdio};
use std::time::Instant;

pub enum Answer<'a> {
    Line(&'a [u8]),
    Eof,
    None,
}

pub fn run(sandbox: &Sandbox, answer: Answer<'_>) -> Output {
    run_args(sandbox, answer, &["setup"])
}

pub fn run_args(sandbox: &Sandbox, answer: Answer<'_>, args: &[&str]) -> Output {
    let mut master = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open("/dev/ptmx")
        .expect("owned pty master");
    let fd = master.as_raw_fd();
    // SAFETY: fd is a live pseudoterminal master and name has 128 writable bytes.
    let mut name = [0i8; 128];
    unsafe {
        assert_eq!(libc::grantpt(fd), 0, "grantpt");
        assert_eq!(libc::unlockpt(fd), 0, "unlockpt");
        assert_eq!(
            libc::ioctl(fd, libc::TIOCPTYGNAME as libc::c_ulong, name.as_mut_ptr()),
            0,
            "slave name"
        );
    }
    assert!(name.contains(&0), "slave path was not terminated");
    // SAFETY: TIOCPTYGNAME returned a nul-terminated path in name.
    let slave_name = unsafe { CStr::from_ptr(name.as_ptr()) };
    let slave = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(OsStr::from_bytes(slave_name.to_bytes()))
        .expect("owned pty slave");
    // SAFETY: slave is a live terminal descriptor and termios is writable.
    let eof = unsafe {
        let mut termios = std::mem::zeroed();
        assert_eq!(libc::tcgetattr(slave.as_raw_fd(), &mut termios), 0);
        termios.c_cc[libc::VEOF]
    };
    let answer = match answer {
        Answer::Line(bytes) => Some(bytes.to_vec()),
        Answer::Eof => Some(vec![eof]),
        Answer::None => None,
    };
    support::capture::nonblocking(&master).expect("nonblocking master");
    let environment = sandbox.vpt();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vpt-fake-engine"));
    command
        .arg("terminal-exec")
        .arg(support::VPT)
        .args(args)
        .stdin(Stdio::from(slave))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(
            environment
                .get_envs()
                .map(|(key, value)| (key, value.expect("set environment"))),
        );
    let started = Instant::now();
    let child = support::spawn(&mut command);
    drop(command);
    let mut seen = Vec::new();
    let mut answered = false;
    let output = support::collect_with(child, started, || {
        let mut buffer = [0; 1024];
        match master.read(&mut buffer) {
            Ok(count) => seen.extend_from_slice(&buffer[..count]),
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.raw_os_error() == Some(libc::EIO) => {}
            Err(error) => panic!("terminal read: {error}"),
        }
        if !answered
            && seen
                .windows(b"choice [1]: ".len())
                .any(|w| w == b"choice [1]: ")
        {
            if let Some(answer) = &answer {
                master.write_all(answer).expect("answer");
            }
            answered = true;
        }
    });
    assert!(
        answered,
        "prompt not observed: {}",
        String::from_utf8_lossy(&seen)
    );
    output
}
