use std::io;
use std::process::{Child, ChildStdin, ExitStatus};

enum Authority {
    Owned,
    ReapingOnly,
    Released,
}

pub(super) struct FixtureChild {
    child: Child,
    authority: Authority,
}

impl FixtureChild {
    pub fn new(child: Child) -> Self {
        Self {
            child,
            authority: Authority::Owned,
        }
    }

    pub fn pid(&self) -> libc::pid_t {
        self.child.id() as libc::pid_t
    }

    pub fn input(&mut self) -> &mut Option<ChildStdin> {
        &mut self.child.stdin
    }

    pub fn terminal(&mut self) -> io::Result<bool> {
        if matches!(self.authority, Authority::Released) {
            return Err(io::Error::from_raw_os_error(libc::ECHILD));
        }
        // SAFETY: a zeroed status is valid output storage for waitid.
        let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
        // SAFETY: this guard owns the exact unreaped child; WNOWAIT retains it.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result == 0 {
            return Ok(info.si_pid != 0);
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ECHILD) {
            self.authority = Authority::Released;
        }
        Err(error)
    }

    pub fn kill(&mut self) -> io::Result<()> {
        if !matches!(self.authority, Authority::Owned) {
            return Err(io::Error::from_raw_os_error(libc::ECHILD));
        }
        self.child.kill()
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        if matches!(self.authority, Authority::Released) {
            return Err(io::Error::from_raw_os_error(libc::ECHILD));
        }
        self.authority = Authority::ReapingOnly;
        loop {
            match self.child.wait() {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(status) => {
                    self.authority = Authority::Released;
                    return Ok(status);
                }
                Err(error) => {
                    if error.raw_os_error() == Some(libc::ECHILD) {
                        self.authority = Authority::Released;
                    }
                    return Err(error);
                }
            }
        }
    }
}

impl Drop for FixtureChild {
    fn drop(&mut self) {
        if matches!(self.authority, Authority::Owned) {
            let _ = self.kill();
        }
        if !matches!(self.authority, Authority::Released) {
            let _ = self.wait();
        }
    }
}
