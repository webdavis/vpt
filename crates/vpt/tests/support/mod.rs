pub(crate) mod capture;
mod process;

use capture::Capture;
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
    collect(spawn(command), started)
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

pub fn collect(child: Child, started: Instant) -> Output {
    collect_with(child, started, || {})
}

pub fn collect_with(child: Child, started: Instant, mut drive: impl FnMut()) -> Output {
    let mut group = ProcessGroup::new(child, started + Duration::from_millis(900));
    let mut stdout = Capture::new(group.child.stdout.take());
    let mut stderr = Capture::new(group.child.stderr.take());
    let status = loop {
        drive();
        stdout.drain().expect("read stdout");
        stderr.drain().expect("read stderr");
        if let Some(status) = group.child.try_wait().expect("wait for command")
            && stdout.finished()
            && stderr.finished()
        {
            break Some(status);
        }
        if started.elapsed() >= Duration::from_millis(500) {
            break None;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    drop(group);
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "command exceeded one second: {elapsed:?}"
    );
    if elapsed > Duration::from_millis(200) {
        eprintln!("slow command: {elapsed:?}");
    }
    Output {
        status: status.expect("command exceeded 500 ms deadline"),
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
