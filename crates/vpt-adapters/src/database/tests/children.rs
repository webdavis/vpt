mod budget;
mod capture;
mod identity;
mod process;
pub(super) mod scenarios;
mod supervision;
mod turn;

use capture::{Capture, nonblocking};
use process::ProcessGroup;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, Output, Stdio};
use std::time::{Duration, Instant};
pub(crate) use turn::turn;

pub(crate) struct ChildGuard {
    group: ProcessGroup,
    input: ChildStdin,
    stdout: Capture<ChildStdout>,
    stderr: Capture<ChildStderr>,
    ready_offset: usize,
    started: Instant,
    deadline: Instant,
    probe: (String, u128),
}

fn probe_epoch() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros()
}

impl ChildGuard {
    fn probe_log(&self, outcome: &str) {
        let Some(path) = std::env::var_os("VPT_CHILD_LOG") else {
            return;
        };
        let text = String::from_utf8_lossy(&self.stderr.bytes);
        let mut sync_ms = 0u128;
        let mut syncs = 0;
        let mut child_epoch = None;
        let mut open_sync = None;
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("VPT_EPOCH_US ") {
                child_epoch = v.parse::<u128>().ok();
            }
            if line.contains("sync.end") {
                syncs += 1;
                open_sync = None;
                sync_ms += line
                    .split("duration_ms=")
                    .nth(1)
                    .and_then(|v| v.trim().parse::<u128>().ok())
                    .unwrap_or(0);
            } else if line.contains("sync.start") {
                open_sync = Some(line.to_owned());
            }
        }
        let start_ms = child_epoch.map(|c| (c as i128 - self.probe.1 as i128) / 1000);
        let family = if self.probe.0.starts_with("ledger_") {
            "ledger"
        } else {
            "database"
        };
        let line = format!(
            "{family} {} {} total_ms={} start_ms={} sync_ms={} syncs={} open_sync={}\n",
            self.probe.0,
            outcome,
            self.started.elapsed().as_millis(),
            start_ms.map_or(-1, |v| v),
            sync_ms,
            syncs,
            open_sync.is_some()
        );
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        file.write_all(line.as_bytes()).unwrap();
    }
}

pub(super) fn child(scenario: &str, directory: &Path) -> io::Result<ChildGuard> {
    spawn(
        scenario,
        directory,
        std::env::var_os("VPT_DATABASE_ROOT_PID"),
        Instant::now(),
    )
}

pub(crate) fn child_for_root(
    scenario: &str,
    directory: &Path,
    root_pid: u32,
    started: Instant,
) -> io::Result<ChildGuard> {
    spawn(
        scenario,
        directory,
        Some(root_pid.to_string().into()),
        started,
    )
}

fn spawn(
    scenario: &str,
    directory: &Path,
    root_pid: Option<std::ffi::OsString>,
    started: Instant,
) -> io::Result<ChildGuard> {
    spawn_with_signal(
        scenario,
        directory,
        root_pid,
        started,
        process::signal_group,
    )
}

fn spawn_with_signal(
    scenario: &str,
    directory: &Path,
    root_pid: Option<std::ffi::OsString>,
    started: Instant,
    signal: impl FnMut(i32, i32) + Send + 'static,
) -> io::Result<ChildGuard> {
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
    if let Some(root_pid) = root_pid {
        command.env("VPT_DATABASE_ROOT_PID", root_pid);
    }
    if std::env::var_os("VPT_PROBE").is_some() {
        command.env("VPT_PROBE", "1");
    }
    // SAFETY: the pre-exec closure uses raw process syscalls and stack-owned values.
    unsafe {
        command.pre_exec(|| {
            let limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_CORE, &limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    let spawn_epoch = probe_epoch();
    let mut group = ProcessGroup::new(
        command.spawn()?,
        started + probe_widened(Duration::from_millis(900)),
        signal,
    );
    let input = group.stdin.take().expect("piped input");
    nonblocking(&input)?;
    let stdout = Capture::new(group.stdout.take().expect("piped stdout"))?;
    let stderr = Capture::new(group.stderr.take().expect("piped stderr"))?;
    Ok(ChildGuard {
        group,
        input,
        stdout,
        stderr,
        ready_offset: 0,
        started,
        deadline: started + probe_widened(operation_budget(scenario)),
        probe: (scenario.to_owned(), spawn_epoch),
    })
}

fn probe_widened(budget: Duration) -> Duration {
    if std::env::var_os("VPT_PROBE").is_some() {
        budget + Duration::from_secs(3)
    } else {
        budget
    }
}

fn operation_budget(scenario: &str) -> Duration {
    if scenario.starts_with("ledger_contract_history_") {
        Duration::from_millis(700)
    } else {
        Duration::from_millis(500)
    }
}

impl ChildGuard {
    fn poll(&mut self) -> io::Result<()> {
        if Instant::now() >= self.deadline {
            return Err(self.timeout());
        }
        self.stdout.drain()?;
        self.stderr.drain()
    }

    fn timeout(&mut self) -> io::Error {
        self.probe_log("TIMEOUT");
        let child = match self.group.observe() {
            Ok(None) => "running".to_owned(),
            Ok(Some(status)) => format!("exited:{status}"),
            Err(error) => format!("wait-error:{:?}", error.kind()),
        };
        let preview = |bytes: &[u8]| bytes[..bytes.len().min(64)].escape_ascii().to_string();
        let phase = self
            .stderr
            .bytes
            .split(|byte| *byte == b'\n')
            .rev()
            .find_map(|line| line.strip_prefix(b"VPT_PHASE "))
            .map_or_else(|| "none".to_owned(), preview);
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "child={child} parent_elapsed_ms={} stdout_eof={} stderr_eof={} stdout_bytes={} stderr_bytes={} phase={phase} stdout_preview={} stderr_preview={}",
                self.started.elapsed().as_millis(),
                self.stdout.finished(),
                self.stderr.finished(),
                self.stdout.bytes.len(),
                self.stderr.bytes.len(),
                preview(&self.stdout.bytes),
                preview(&self.stderr.bytes),
            ),
        )
    }

    pub(crate) fn ready(&mut self) -> io::Result<()> {
        loop {
            self.poll()?;
            for line in
                self.stdout.bytes[self.ready_offset..].split_inclusive(|byte| *byte == b'\n')
            {
                if !line.ends_with(b"\n") {
                    break;
                }
                self.ready_offset += line.len();
                if line == b"VPT_READY\n" {
                    return Ok(());
                }
            }
            if self.group.observe()?.is_some() {
                return Err(io::Error::other(format!(
                    "child exited before ready: {:?}",
                    self.stderr.bytes
                )));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    pub(crate) fn send(&mut self, command: &str) -> io::Result<()> {
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

    pub(crate) fn finish(mut self) -> io::Result<Output> {
        loop {
            self.poll()?;
            if let Some(status) = self.group.observe()?
                && self.stdout.finished()
                && self.stderr.finished()
            {
                self.probe_log("ok");
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
