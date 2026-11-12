pub(crate) mod capture;
mod diagnostic;
pub(crate) mod process;
mod turn;

use capture::Capture;
use diagnostic::Diagnostic;
pub(crate) use process::ProcessGroup;
use std::net::Shutdown;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

pub const VPT: &str = env!("CARGO_BIN_EXE_vpt");
/// Bounds a command or test child so that a hang fails its test. It does not measure
/// performance: commands finish far inside it, and the margin absorbs a CI runner stalling the
/// whole machine.
pub const WATCHDOG: Duration = Duration::from_secs(2);
/// Bounds the forced cleanup of a command that outlived the watchdog.
pub const CLEANUP_BOUND: Duration = WATCHDOG.saturating_add(Duration::from_millis(400));
#[cfg(feature = "dev-tools")]
#[allow(dead_code, reason = "used only by helper integration tests")]
pub const FAKE_ENGINE: &str = env!("CARGO_BIN_EXE_vpt-fake-engine");

pub fn broken_output() -> (OwnedFd, UnixStream) {
    let (writer, peer) = UnixStream::pair().expect("local socket pair");
    writer
        .shutdown(Shutdown::Write)
        .expect("disable socket writes");
    (writer.into(), peer)
}

#[allow(dead_code, reason = "shared fixture used by mutating command tests")]
pub fn run_with_broken_stdout(command: &mut Command, budget: Duration) -> Output {
    let (descriptor, _peer) = broken_output();
    command.stdout(Stdio::from(descriptor));
    run_with_budget(command, budget)
}

pub struct Sandbox {
    root: tempfile::TempDir,
}

impl Sandbox {
    pub fn new(name: &str) -> Self {
        let root = tempfile::Builder::new()
            .prefix(&format!("vpt-{name}-"))
            .tempdir()
            .expect("sandbox");
        for leaf in [
            "home",
            "config",
            "data",
            "state",
            "cache",
            "tmp",
            "bin",
            "voice-memos/Recordings",
        ] {
            std::fs::create_dir_all(root.path().join(leaf)).expect("sandbox leaf");
        }
        Self { root }
    }

    pub fn path(&self) -> &Path {
        self.root.path()
    }

    pub fn config_path(&self) -> PathBuf {
        self.path().join("config/vpt/config.toml")
    }

    #[cfg(feature = "dev-tools")]
    #[allow(dead_code, reason = "used only by helper integration tests")]
    pub fn install_fake_helper(&self) {
        std::os::unix::fs::symlink(FAKE_ENGINE, self.path().join("bin/vpt-macos"))
            .expect("fake helper");
        std::fs::create_dir_all(self.path().join("trash")).expect("fake trash");
    }

    pub fn fake_log(&self) -> PathBuf {
        self.path().join("fake.log")
    }

    pub fn vpt(&self) -> Command {
        let mut command = Command::new(VPT);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .env("HOME", self.path().join("home"))
            .env("XDG_CONFIG_HOME", self.path().join("config"))
            .env("XDG_DATA_HOME", self.path().join("data"))
            .env("XDG_STATE_HOME", self.path().join("state"))
            .env("XDG_CACHE_HOME", self.path().join("cache"))
            .env("TMPDIR", self.path().join("tmp"))
            .env("VPT_CONFIG", self.config_path())
            .env("PATH", self.path().join("bin"))
            .env("VPT_FAKE_TRASH", self.path().join("trash"))
            .env("VPT_FAKE_LOG", self.fake_log());
        command
    }
}

pub fn run(command: &mut Command) -> Output {
    let _turn = turn::turn();
    let started = Instant::now();
    let child = spawn(command);
    let spawn_return = Instant::now();
    collect(child, started, Some(spawn_return))
}

pub fn spawn(command: &mut Command) -> ProcessGroup {
    spawn_with_signal(command, Instant::now(), process::signal_group)
}

pub(crate) fn spawn_with_signal(
    command: &mut Command,
    started: Instant,
    signal: impl FnMut(i32, i32) + Send + 'static,
) -> ProcessGroup {
    // SAFETY: setsid is async-signal-safe in the child before exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    ProcessGroup::new(
        command.spawn().expect("command spawned"),
        started + CLEANUP_BOUND,
        signal,
    )
}

pub fn collect(child: ProcessGroup, started: Instant, spawn_return: Option<Instant>) -> Output {
    collect_with(child, started, spawn_return, || {})
}

pub fn collect_with(
    group: ProcessGroup,
    started: Instant,
    spawn_return: Option<Instant>,
    drive: impl FnMut(),
) -> Output {
    collect_with_budget(group, started, spawn_return, drive, WATCHDOG)
}

#[allow(
    dead_code,
    reason = "durable recovery scenarios allow multiple filesystem synchronizations"
)]
pub fn run_with_budget(command: &mut Command, budget: Duration) -> Output {
    let _turn = turn::turn();
    let started = Instant::now();
    let child = spawn(command);
    collect_with_budget(child, started, Some(Instant::now()), || {}, budget)
}

fn collect_with_budget(
    mut group: ProcessGroup,
    started: Instant,
    spawn_return: Option<Instant>,
    mut drive: impl FnMut(),
    budget: Duration,
) -> Output {
    let mut diagnostic = Diagnostic::new(started, group.id(), spawn_return);
    group.set_deadline(started + budget + Duration::from_millis(400));
    let stdout_piped = group.stdout.is_some();
    let stderr_piped = group.stderr.is_some();
    let mut stdout = Capture::new(group.stdout.take());
    let mut stderr = Capture::new(group.stderr.take());
    let status = loop {
        let poll_started = Instant::now();
        diagnostic.poll_started(poll_started);
        if poll_started.duration_since(started) >= budget {
            diagnostic.deadline(poll_started);
            break None;
        }
        let step_started = Instant::now();
        drive();
        diagnostic.step("driver", step_started, Instant::now());
        let step_started = Instant::now();
        stdout.drain().expect("read stdout");
        let step_done = Instant::now();
        diagnostic.step("stdout_drain", step_started, step_done);
        if stdout_piped && stdout.finished() {
            diagnostic.stdout_eof(step_done);
        }
        let step_started = Instant::now();
        stderr.drain().expect("read stderr");
        let step_done = Instant::now();
        diagnostic.step("stderr_drain", step_started, step_done);
        if stderr_piped && stderr.finished() {
            diagnostic.stderr_eof(step_done);
        }
        let step_started = Instant::now();
        let observed = group.observe().expect("wait for command");
        let observed_at = Instant::now();
        diagnostic.step("try_wait", step_started, observed_at);
        if let Some(status) = observed {
            diagnostic.child_exit(observed_at, status);
        }
        if observed_at.duration_since(started) >= budget {
            diagnostic.deadline(observed_at);
            break None;
        }
        if observed.is_some() && stdout.finished() && stderr.finished() {
            break observed;
        }
        diagnostic.poll_completed(observed_at);
        std::thread::sleep(Duration::from_millis(1));
    };
    drop(group);
    let cleaned_at = Instant::now();
    diagnostic.cleanup(cleaned_at);
    let elapsed = cleaned_at.duration_since(started);
    if elapsed >= budget + Duration::from_millis(500) || status.is_none() {
        let record = diagnostic.render(
            &stdout.bytes,
            &stderr.bytes,
            stdout.finished(),
            stderr.finished(),
        );
        panic!(
            "command exceeded {} ms deadline: {record}",
            budget.as_millis()
        );
    }
    if elapsed > Duration::from_millis(200) {
        eprintln!("slow command: {elapsed:?}");
    }
    Output {
        status: status.expect("completed command status"),
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    }
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
