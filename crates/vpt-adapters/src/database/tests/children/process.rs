use std::io::{self, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, ExitStatus};
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) enum Identity {
    #[default]
    Held,
    Lost,
}

impl Identity {
    pub(super) fn observe_with(
        &mut self,
        observe: impl FnOnce() -> io::Result<Option<ExitStatus>>,
    ) -> io::Result<Option<ExitStatus>> {
        if matches!(self, Self::Lost) {
            return Err(io::Error::from_raw_os_error(libc::ECHILD));
        }
        let result = observe();
        if matches!(&result, Err(error) if error.raw_os_error() == Some(libc::ECHILD)) {
            *self = Self::Lost;
        }
        result
    }
}

pub(super) fn decode_status(event: i32, status: i32) -> io::Result<ExitStatus> {
    let raw = match event {
        libc::CLD_EXITED => (status & 0xff) << 8,
        libc::CLD_KILLED => status,
        libc::CLD_DUMPED => status | 0x80,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected nonterminal wait event",
            ));
        }
    };
    Ok(ExitStatus::from_raw(raw))
}

pub(super) fn cleanup_observation(
    observed: io::Result<Option<ExitStatus>>,
    first_error: &mut Option<io::Error>,
) -> io::Result<bool> {
    match observed {
        Err(error) if error.raw_os_error() == Some(libc::ECHILD) => Err(error),
        Err(error) => {
            first_error.get_or_insert(error);
            Ok(false)
        }
        Ok(status) => Ok(status.is_none()),
    }
}

pub(super) struct ProcessGroup {
    child: Child,
    pub(super) stdin: Option<ChildStdin>,
    pub(super) stdout: Option<ChildStdout>,
    pub(super) stderr: Option<ChildStderr>,
    deadline: Instant,
    identity: Identity,
    signal: Box<dyn FnMut(i32, i32) + Send>,
}

impl ProcessGroup {
    pub(super) fn new(
        mut child: Child,
        deadline: Instant,
        signal: impl FnMut(i32, i32) + Send + 'static,
    ) -> Self {
        Self {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            child,
            deadline,
            identity: Identity::Held,
            signal: Box::new(signal),
        }
    }

    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    pub(super) fn cleanup_deadline(&self) -> Instant {
        self.deadline
    }

    pub(super) fn observe(&mut self) -> io::Result<Option<ExitStatus>> {
        let pid = self.child.id();
        self.identity.observe_with(|| {
            // SAFETY: siginfo_t is a plain C struct for which all-zero bytes are valid.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            // SAFETY: zeroed storage receives this owned child's status without reaping it.
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    pid as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if result == -1 {
                return Err(io::Error::last_os_error());
            }
            if info.si_pid == 0 {
                return Ok(None);
            }
            if u32::try_from(info.si_pid).ok() != Some(pid) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "wait returned another child",
                ));
            }
            decode_status(info.si_code, info.si_status).map(Some)
        })
    }

    fn cleanup(&mut self) -> io::Result<()> {
        let mut observation_error = None;
        let running = cleanup_observation(self.observe(), &mut observation_error)?;
        let pid = i32::try_from(self.child.id()).expect("child pid fits pid_t");
        (self.signal)(pid, libc::SIGTERM);
        let reap_from = self
            .deadline
            .checked_sub(super::GRACE)
            .unwrap_or(self.deadline);
        let grace = (Instant::now() + super::GRACE).min(reap_from);
        while running
            && cleanup_observation(self.observe(), &mut observation_error)?
            && Instant::now() < grace
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        (self.signal)(pid, libc::SIGKILL);
        let waited = loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break Ok(()),
                Err(error) if error.kind() != io::ErrorKind::Interrupted => break Err(error),
                _ if Instant::now() >= self.deadline => break Err(io::ErrorKind::TimedOut.into()),
                _ => std::thread::sleep(Duration::from_millis(1)),
            }
        };
        if let Some(error) = observation_error {
            waited.map_err(|wait_error| {
                io::Error::new(
                    wait_error.kind(),
                    format!("{wait_error}; earlier observation error: {error}"),
                )
            })?;
            return Err(error);
        }
        waited
    }
}

pub(super) fn signal_group(pid: i32, signal: i32) {
    // SAFETY: the guard retains its spawned group leader until signaling ends.
    unsafe { libc::killpg(pid, signal) };
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            if std::thread::panicking() {
                let _ = writeln!(io::stderr(), "owned child cleanup failed: {error}");
            } else {
                panic!("owned child cleanup failed: {error}");
            }
        }
    }
}
