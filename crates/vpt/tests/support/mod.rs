pub(crate) mod capture;
mod diagnostic;
mod process;

use capture::Capture;
use diagnostic::Diagnostic;
use process::ProcessGroup;
use std::net::Shutdown;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

pub const VPT: &str = env!("CARGO_BIN_EXE_vpt");

pub fn broken_output() -> (OwnedFd, UnixStream) {
    let (writer, peer) = UnixStream::pair().expect("local socket pair");
    writer
        .shutdown(Shutdown::Write)
        .expect("disable socket writes");
    (writer.into(), peer)
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
            .env("VPT_CONFIG", self.config_path())
            .env("PATH", self.path().join("bin"));
        command
    }
}

pub fn run(command: &mut Command) -> Output {
    let started = Instant::now();
    let child = spawn(command);
    let spawn_return = Instant::now();
    collect(child, started, Some(spawn_return))
}

pub fn spawn(command: &mut Command) -> Child {
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
    command.spawn().expect("command spawned")
}

pub fn collect(child: Child, started: Instant, spawn_return: Option<Instant>) -> Output {
    collect_with(child, started, spawn_return, || {})
}

pub fn collect_with(
    child: Child,
    started: Instant,
    spawn_return: Option<Instant>,
    mut drive: impl FnMut(),
) -> Output {
    let mut diagnostic = Diagnostic::new(started, child.id(), spawn_return);
    let mut group = ProcessGroup::new(child, started + Duration::from_millis(900));
    let stdout_piped = group.child.stdout.is_some();
    let stderr_piped = group.child.stderr.is_some();
    let mut stdout = Capture::new(group.child.stdout.take());
    let mut stderr = Capture::new(group.child.stderr.take());
    let status = loop {
        let poll_started = Instant::now();
        diagnostic.poll_started(poll_started);
        if poll_started.duration_since(started) >= Duration::from_millis(500) {
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
        let observed = group.child.try_wait().expect("wait for command");
        let observed_at = Instant::now();
        diagnostic.step("try_wait", step_started, observed_at);
        if let Some(status) = observed {
            diagnostic.child_exit(observed_at, status);
        }
        if observed_at.duration_since(started) >= Duration::from_millis(500) {
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
    if elapsed >= Duration::from_secs(1) || status.is_none() {
        let record = diagnostic.render(
            &stdout.bytes,
            &stderr.bytes,
            stdout.finished(),
            stderr.finished(),
        );
        if elapsed >= Duration::from_secs(1) {
            panic!("command exceeded one second: {record}");
        }
        panic!("command exceeded 500 ms deadline: {record}");
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
