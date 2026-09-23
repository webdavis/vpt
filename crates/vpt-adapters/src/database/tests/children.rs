mod capture;
mod process;
mod scenarios;

use capture::{Capture, nonblocking};
use process::ProcessGroup;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, Output, Stdio};
use std::time::{Duration, Instant};

pub(super) struct ChildGuard {
    group: ProcessGroup,
    input: ChildStdin,
    stdout: Capture<ChildStdout>,
    stderr: Capture<ChildStderr>,
    deadline: Instant,
}

pub(super) fn child(scenario: &str, directory: &Path) -> io::Result<ChildGuard> {
    let started = Instant::now();
    let directory = directory.canonicalize()?;
    for leaf in ["home", "config", "data", "xdg-state", "bin"] {
        std::fs::create_dir_all(directory.join(leaf))?;
    }
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "database::tests::children::scenarios::database_child",
            "--nocapture",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(&directory)
        .env_clear()
        .env("HOME", directory.join("home"))
        .env("XDG_CONFIG_HOME", directory.join("config"))
        .env("XDG_DATA_HOME", directory.join("data"))
        .env("XDG_STATE_HOME", directory.join("xdg-state"))
        .env("VPT_CONFIG", directory.join("config/vpt.toml"))
        .env("PATH", directory.join("bin"))
        .env("VPT_DATABASE_SCENARIO", scenario)
        .env("VPT_DATABASE_ROOT", &directory);
    // SAFETY: setsid is async-signal-safe in the child before exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let mut group = ProcessGroup::new(command.spawn()?, started + Duration::from_millis(900));
    let input = group.child.stdin.take().expect("piped input");
    nonblocking(&input)?;
    let stdout = Capture::new(group.child.stdout.take().expect("piped stdout"))?;
    let stderr = Capture::new(group.child.stderr.take().expect("piped stderr"))?;
    Ok(ChildGuard {
        group,
        input,
        stdout,
        stderr,
        deadline: started + Duration::from_millis(500),
    })
}

impl ChildGuard {
    fn poll(&mut self) -> io::Result<()> {
        if Instant::now() >= self.deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.stdout.drain()?;
        self.stderr.drain()
    }

    pub(super) fn ready(&mut self) -> io::Result<()> {
        loop {
            self.poll()?;
            if self
                .stdout
                .bytes
                .split(|byte| *byte == b'\n')
                .any(|line| line == b"VPT_READY")
            {
                return Ok(());
            }
            if self.group.child.try_wait()?.is_some() {
                return Err(io::Error::other(format!(
                    "child exited before ready: {:?}",
                    self.stderr.bytes
                )));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    pub(super) fn send(&mut self, command: &str) -> io::Result<()> {
        if command != "continue" {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut bytes = b"continue\n".as_slice();
        while !bytes.is_empty() {
            self.poll()?;
            match self.input.write(bytes) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => bytes = &bytes[count..],
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> io::Result<Output> {
        loop {
            self.poll()?;
            if let Some(status) = self.group.child.try_wait()?
                && self.stdout.finished()
                && self.stderr.finished()
            {
                return Ok(Output {
                    status,
                    stdout: std::mem::take(&mut self.stdout.bytes),
                    stderr: std::mem::take(&mut self.stderr.bytes),
                });
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
