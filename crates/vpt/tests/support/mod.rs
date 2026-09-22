use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

pub const VPT: &str = env!("CARGO_BIN_EXE_vpt");

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
    let mut group = ProcessGroup(command.process_group(0).spawn().expect("command spawned"));
    let out = group.0.stdout.take();
    let err = group.0.stderr.take();
    let stdout = std::thread::spawn(move || read_output(out));
    let stderr = std::thread::spawn(move || read_output(err));
    let status = loop {
        match group.0.try_wait() {
            Ok(Some(status)) => break Ok(Some(status)),
            Ok(None) if started.elapsed() < Duration::from_millis(500) => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(None) => break Ok(None),
            Err(error) => break Err(error),
        }
    };
    drop(group);
    let stdout = stdout.join().expect("stdout reader").expect("read stdout");
    let stderr = stderr.join().expect("stderr reader").expect("read stderr");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(1),
        "command exceeded one second: {elapsed:?}"
    );
    if elapsed > Duration::from_millis(200) {
        eprintln!("slow command: {elapsed:?}");
    }
    Output {
        status: status
            .expect("wait for command")
            .expect("command exceeded 500 ms deadline"),
        stdout,
        stderr,
    }
}

fn read_output(pipe: Option<impl Read>) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    if let Some(mut pipe) = pipe {
        pipe.read_to_end(&mut bytes)?;
    }
    Ok(bytes)
}

struct ProcessGroup(Child);

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        let pid = i32::try_from(self.0.id()).expect("child pid fits pid_t");
        // The child created this process group; only its descendants receive signals.
        unsafe { libc::killpg(pid, libc::SIGTERM) };
        let deadline = Instant::now() + Duration::from_millis(50);
        while matches!(self.0.try_wait(), Ok(None)) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        // Kill surviving descendants even when the group leader has exited.
        unsafe { libc::killpg(pid, libc::SIGKILL) };
        self.0.wait().expect("reap child");
    }
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
