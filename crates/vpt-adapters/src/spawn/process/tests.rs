mod decoder;

use super::super::{lease::SignalLease, reap};
use super::*;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Instant;

enum Ownership {
    Owned,
    ReapingOnly,
    Reaped,
    Lost,
}

struct OwnedLeader {
    child: Child,
    lease: SignalLease,
    ownership: Ownership,
}

impl OwnedLeader {
    fn new(root: &Path) -> Self {
        let child = Command::new("/bin/sh")
            .args(["-c", "exit 3"])
            .env_clear()
            .env("HOME", root.join("home"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_STATE_HOME", root.join("state"))
            .env("VPT_CONFIG", root.join("config.toml"))
            .env("PATH", root.join("bin"))
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .expect("owned finite leader");
        let lease = SignalLease::new(child.id() as libc::pid_t);
        Self {
            child,
            lease,
            ownership: Ownership::Owned,
        }
    }

    fn pid(&self) -> libc::pid_t {
        self.lease.group()
    }

    fn observe(&mut self) -> io::Result<Option<Status>> {
        let result = self.lease.observe();
        if result
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(libc::ECHILD))
        {
            self.ownership = Ownership::Lost;
        }
        result
    }

    fn only_terminal_leader(&mut self) -> io::Result<bool> {
        let result = self.lease.only_terminal_leader();
        if result
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(libc::ECHILD))
        {
            self.ownership = Ownership::Lost;
        }
        result
    }

    fn finish(&mut self, kill: bool) -> io::Result<Option<ExitStatus>> {
        let mut failure = None;
        match self.ownership {
            Ownership::Owned => {
                if kill {
                    match self.lease.signal(libc::SIGKILL) {
                        Err(error) if error.raw_os_error() == Some(libc::ECHILD) => {
                            self.ownership = Ownership::Lost;
                            return Err(error);
                        }
                        Err(error) => failure = Some(error),
                        Ok(()) => {}
                    }
                }
                self.lease.revoke();
                self.ownership = Ownership::ReapingOnly;
            }
            Ownership::Lost => return Err(io::Error::from_raw_os_error(libc::ECHILD)),
            Ownership::Reaped => return Ok(None),
            Ownership::ReapingOnly => {}
        }
        loop {
            match reap::finish(
                failure.take(),
                || self.child.try_wait(),
                std::thread::yield_now,
            ) {
                Ok(reaped) => {
                    self.ownership = Ownership::Reaped;
                    return reaped.failure.map_or(Ok(Some(reaped.status)), Err);
                }
                Err(error) if error.raw_os_error() == Some(libc::ECHILD) => {
                    self.ownership = Ownership::Lost;
                    return Err(error);
                }
                Err(error) => {
                    failure.get_or_insert(error);
                }
            }
        }
    }

    fn reap(&mut self) {
        let status = self
            .finish(false)
            .expect("exact child reap")
            .expect("owned child status");
        assert_eq!(status.code(), Some(3));
        // SAFETY: this initialized observation cannot consume or reacquire the recorded child.
        let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
        assert_eq!(
            // SAFETY: WNOWAIT is only an observation after successful exact reaping.
            unsafe {
                libc::waitid(
                    libc::P_PID,
                    self.pid() as libc::id_t,
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
}

impl Drop for OwnedLeader {
    fn drop(&mut self) {
        let _ = self.finish(true);
    }
}

fn until_terminal(child: &mut OwnedLeader, started: Instant) -> Status {
    loop {
        assert!(
            started.elapsed() < crate::database::WATCHDOG,
            "finite leader readiness exceeded real deadline"
        );
        match child.observe() {
            Ok(Some(status)) => return status,
            Ok(None) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => panic!("non-reaping status: {:?}", error.kind()),
        }
        std::thread::yield_now();
    }
}

#[test]
fn unreaped_exited_leader_group_presence_probe() {
    let root = crate::test_temp::tempdir().expect("owned root");
    let started = Instant::now();
    let mut child = OwnedLeader::new(root.path());
    let status = until_terminal(&mut child, started);
    // SAFETY: signal zero observes the still-reserved group without sending a signal.
    let presence = unsafe { libc::killpg(child.pid(), 0) };
    let error = if presence == -1 {
        io::Error::last_os_error().raw_os_error()
    } else {
        None
    };
    eprintln!(
        "unreaped leader: pid={} status={status:?} group_presence={presence} errno={error:?}",
        child.pid()
    );
    assert_eq!(status, Status::Exited(3));
    assert!(
        presence == 0 || matches!(error, Some(libc::EPERM | libc::ESRCH)),
        "unexpected owned-group observation"
    );
    assert!(
        child
            .only_terminal_leader()
            .expect("complete owned-group membership")
    );
    assert_eq!(
        child.observe().expect("still waitable"),
        Some(Status::Exited(3))
    );
    child.reap();
    // SAFETY: signal zero only observes the recorded group and never restores authority.
    assert_eq!(unsafe { libc::killpg(child.pid(), 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    assert!(started.elapsed() < crate::database::CLEANUP_BOUND);
}

#[test]
fn terminal_observation_preserves_the_owned_leaders_waitable_identity() {
    let root = crate::test_temp::tempdir().expect("owned root");
    let started = Instant::now();
    let mut child = OwnedLeader::new(root.path());
    until_terminal(&mut child, started);
    let observed = child.observe();
    let retained = child.observe();
    assert_eq!(
        observed.expect("terminal observation"),
        Some(Status::Exited(3))
    );
    assert_eq!(
        retained.expect("observer must retain identity"),
        Some(Status::Exited(3))
    );
    child.reap();
    assert!(started.elapsed() < crate::database::CLEANUP_BOUND);
}
