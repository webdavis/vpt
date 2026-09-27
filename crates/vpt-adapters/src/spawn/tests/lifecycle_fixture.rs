use super::super::{lease::SignalLease, runtime::PipeSlots};
use super::{peers::Peer, watchdog::Registration};
use std::cell::{Cell, RefCell};
use std::io;
use std::os::unix::net::UnixStream;
use std::process::{Command, ExitStatus};
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

pub(super) struct Fixture {
    pub root: tempfile::TempDir,
    pub started: Instant,
    pub group: Cell<libc::pid_t>,
    peer: Cell<libc::pid_t>,
    active: Rc<Cell<bool>>,
    fired: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    reaped: Cell<Option<ExitStatus>>,
    lease: RefCell<Option<SignalLease>>,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            root: tempfile::tempdir().expect("owned root"),
            started: Instant::now(),
            group: Cell::new(0),
            peer: Cell::new(0),
            active: Rc::new(Cell::new(false)),
            fired: Arc::new(AtomicBool::new(false)),
            failed: Arc::new(AtomicBool::new(false)),
            reaped: Cell::new(None),
            lease: RefCell::new(None),
        }
    }

    pub fn command(&self, program: &str, args: &[&str]) -> Command {
        let root = self.root.path();
        let mut command = Command::new(program);
        command
            .args(args)
            .env_clear()
            .current_dir(root)
            .env("HOME", root.join("home"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_STATE_HOME", root.join("state"))
            .env("VPT_CONFIG", root.join("config.toml"))
            .env("PATH", root.join("bin"));
        command
    }

    pub fn pending(&self) -> Command {
        let exe = std::env::current_exe().expect("test executable");
        let mut command = self.command(
            exe.to_str().expect("fixture path"),
            &[
                "--exact",
                "spawn::tests::lifecycle_fixture::pending_leader",
                "--nocapture",
            ],
        );
        command.env("VPT_PROCESS_FIXTURE", self.root.path());
        command
    }

    pub fn ready(&self) -> bool {
        self.root.path().join("ready").exists()
    }

    pub fn leader_terminal(&self) -> bool {
        assert!(self.active.get(), "owned group registration");
        let result = self
            .lease
            .borrow()
            .as_ref()
            .expect("registered signal lease")
            .observe();
        match result {
            Ok(status) => status.is_some(),
            Err(error) => {
                assert_eq!(error.kind(), io::ErrorKind::Interrupted);
                false
            }
        }
    }

    pub fn register(
        &self,
        lease: SignalLease,
        _pipes: &mut PipeSlots,
        registration: &mut Option<Registration>,
    ) -> io::Result<()> {
        self.group.set(lease.group());
        self.lease.replace(Some(lease.clone()));
        *registration = Some(Registration::new(
            lease,
            self.started,
            self.fired.clone(),
            self.active.clone(),
            self.failed.clone(),
        )?);
        Ok(())
    }

    pub fn watchdog_fired(&self) -> bool {
        self.fired.load(Ordering::SeqCst)
    }

    pub fn register_peer(
        &self,
        lease: SignalLease,
        pipes: &mut PipeSlots,
        registration: &mut Option<Registration>,
        closed: bool,
    ) -> io::Result<()> {
        let group = lease.group();
        self.register(lease, pipes, registration)?;
        let (read, write) = UnixStream::pair()?;
        pipes.stdout = Some(read.into());
        let peer = Peer::new(self.root.path(), write.into(), closed, self.started)?;
        self.peer.set(peer.pid());
        registration
            .as_mut()
            .expect("installed registration")
            .attach_peer(peer, group)?;
        while !self.ready() && !self.watchdog_fired() {
            std::thread::yield_now();
        }
        if !self.ready() {
            return Err(io::Error::from(io::ErrorKind::TimedOut));
        }
        Ok(())
    }

    pub fn assert_participant_reaped(&self) {
        let root = self.root.path();
        let ready = std::fs::read_to_string(root.join("ready")).expect("participant readiness");
        let reaped = std::fs::read_to_string(root.join("reaped")).expect("exact reaping record");
        assert_eq!(reaped, format!("{} {} false", ready, libc::SIGKILL));
        assert_reaped(self.peer.get());
    }

    pub fn record_reaped(&self, status: ExitStatus) {
        self.reaped.set(Some(status));
    }

    pub fn recorded_status(&self) -> Option<ExitStatus> {
        self.reaped.get()
    }

    pub fn assert_no_fallback(&self) {
        let status = self.reaped.get().expect("exact status recorded");
        assert!(
            !matches!(status.code(), Some(75 | 76)),
            "child-local fallback exited"
        );
        assert!(
            !self
                .root
                .path()
                .join("fallback")
                .try_exists()
                .expect("fallback marker observation"),
            "child-local fallback marker present"
        );
    }

    pub fn assert_finished(&self) {
        assert!(
            !self.active.get(),
            "registration retained after executor return"
        );
        assert!(!self.watchdog_fired(), "independent watchdog fired");
        assert!(
            !self.failed.load(Ordering::SeqCst),
            "independent cleanup failed"
        );
        let group = self.group.get();
        assert!(group > 1, "owned group was never registered");
        assert_reaped(group);
        // SAFETY: signal zero is a read-only observation of the recorded group number.
        assert_eq!(unsafe { libc::killpg(group, 0) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
        self.assert_no_fallback();
    }
}

fn assert_reaped(pid: libc::pid_t) {
    assert!(pid > 1, "exact owned child identifier");
    // SAFETY: this numeric observation cannot consume or reacquire a child.
    let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
    // SAFETY: WNOWAIT observes the recorded identifier without reaping it.
    assert_eq!(
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
}

#[test]
fn pending_leader() {
    let Some(root) = std::env::var_os("VPT_PROCESS_FIXTURE") else {
        return;
    };
    let started = Instant::now();
    let lifetime = Duration::from_millis(600);
    // SAFETY: this post-exec fixture changes only its own TERM disposition.
    unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
    std::fs::write(std::path::Path::new(&root).join("ready"), b"ready").expect("owned readiness");
    while let Some(remaining) = lifetime.checked_sub(started.elapsed()) {
        std::thread::park_timeout(remaining);
    }
    let marker = std::path::Path::new(&root).join("fallback");
    std::process::exit(if std::fs::write(marker, b"expired").is_ok() {
        75
    } else {
        76
    });
}
