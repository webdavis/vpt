use super::*;
use std::os::unix::ffi::OsStrExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        self.0.wait().expect("reap owned FIFO test process");
    }
}

#[test]
fn opening_a_fifo_returns_not_regular_without_a_writer() {
    const LEAF: &str =
        "contained::tests::fifo::opening_a_fifo_returns_not_regular_without_a_writer";
    const CHILD_ROOT: &str = "VPT_CONTAINED_FIFO_CHILD";
    if let Some(path) = std::env::var_os(CHILD_ROOT) {
        let root = RootDir::open(Path::new(&path)).expect("root");
        assert_eq!(
            root.stat(Path::new("fifo")).expect("stat").kind,
            Kind::Other
        );
        for access in [Access::Read, Access::Write, Access::ReadWrite] {
            assert_eq!(
                root.open_file(Path::new("fifo"), access).err(),
                Some(ContainedError::NotRegular(root.path().join("fifo")))
            );
        }
        return;
    }
    let temp = tempfile::tempdir().expect("owned fixture");
    let path = temp.path().canonicalize().expect("canonical");
    let fifo = CString::new(path.join("fifo").as_os_str().as_bytes()).expect("name");
    // SAFETY: fifo is an owned, NUL-terminated pathname that outlives mkfifo.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let started = Instant::now();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", LEAF, "--nocapture"])
            .env_clear()
            .env("HOME", &path)
            .env("PATH", &path)
            .env(CHILD_ROOT, &path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("owned child"),
    );
    let outcome = loop {
        match child.0.try_wait().expect("child status") {
            Some(status) => break Some(status),
            None if started.elapsed() < crate::database::WATCHDOG => std::thread::yield_now(),
            None => break None,
        }
    };
    drop(child);
    let elapsed = started.elapsed();
    assert!(
        elapsed < crate::database::WATCHDOG.saturating_add(Duration::from_millis(500)),
        "FIFO test exceeded its cleanup bound: {elapsed:?}"
    );
    assert!(
        outcome.is_some(),
        "FIFO rejection exceeded the watchdog; child killed and reaped ({elapsed:?})"
    );
    assert!(
        outcome.expect("completed child").success(),
        "FIFO child failed"
    );
}
