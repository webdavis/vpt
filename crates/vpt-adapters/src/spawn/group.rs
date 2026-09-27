use super::{
    SpawnError, Status,
    lease::SignalLease,
    reap,
    runtime::{PipeSlots, safe_error},
};
use std::io;
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

enum Ownership {
    Owned,
    ReapingOnly,
    Reaped,
    Lost,
}

pub(super) struct OwnedGroup<R, F: Fn(ExitStatus)> {
    child: Child,
    lease: SignalLease,
    registration: Option<R>,
    ownership: Ownership,
    first_failure: Option<io::Error>,
    grace: Duration,
    reaped: F,
}

impl<R, F: Fn(ExitStatus)> OwnedGroup<R, F> {
    pub(super) fn new(child: Child, grace: Duration, reaped: F) -> Self {
        let lease = SignalLease::new(child.id() as libc::pid_t);
        Self {
            child,
            lease,
            registration: None,
            ownership: Ownership::Owned,
            first_failure: None,
            grace,
            reaped,
        }
    }

    pub(super) fn group(&self) -> libc::pid_t {
        self.lease.group()
    }

    pub(super) fn observe(&mut self) -> io::Result<Option<Status>> {
        let result = self.lease.observe();
        if result
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(libc::ECHILD))
        {
            self.ownership = Ownership::Lost;
        }
        result
    }

    pub(super) fn register<O>(&mut self, observed: O) -> io::Result<PipeSlots>
    where
        O: FnOnce(SignalLease, &mut PipeSlots, &mut Option<R>) -> io::Result<()>,
    {
        let mut pipes = PipeSlots {
            stdin: self.child.stdin.take().map(Into::into),
            stdout: self.child.stdout.take().map(Into::into),
            stderr: self.child.stderr.take().map(Into::into),
        };
        observed(self.lease.clone(), &mut pipes, &mut self.registration)?;
        Ok(pipes)
    }

    pub(super) fn remember(&mut self, error: io::Error) {
        if error.raw_os_error() == Some(libc::ECHILD) {
            self.lease.revoke();
            self.ownership = Ownership::Lost;
        }
        self.first_failure.get_or_insert(error);
    }

    fn only_terminal_leader(&mut self) -> bool {
        match self.lease.only_terminal_leader() {
            Ok(only) => only,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => false,
            Err(error) => {
                self.remember(error);
                false
            }
        }
    }

    fn send(&mut self, signal: libc::c_int) {
        if let Err(error) = self.lease.signal(signal) {
            self.remember(error);
        }
    }

    fn stop<K, W>(&mut self, now: &K, wait: &W)
    where
        K: Fn() -> Instant,
        W: Fn(Duration),
    {
        if self.only_terminal_leader() || matches!(self.ownership, Ownership::Lost) {
            return;
        }
        self.send(libc::SIGTERM);
        let started = now();
        while matches!(self.ownership, Ownership::Owned) {
            if self.only_terminal_leader() || matches!(self.ownership, Ownership::Lost) {
                return;
            }
            if now().saturating_duration_since(started) >= self.grace {
                self.send(libc::SIGKILL);
                return;
            }
            wait(Duration::from_millis(1));
        }
    }

    pub(super) fn finish<K, W>(&mut self, now: K, wait: W) -> io::Result<()>
    where
        K: Fn() -> Instant,
        W: Fn(Duration),
    {
        if matches!(self.ownership, Ownership::Owned) {
            self.stop(&now, &wait);
        }
        self.lease.revoke();
        if matches!(self.ownership, Ownership::Owned) {
            self.ownership = Ownership::ReapingOnly;
        }
        drop(self.registration.take());
        match self.ownership {
            Ownership::Lost => Err(io::Error::from_raw_os_error(libc::ECHILD)),
            Ownership::Reaped => Ok(()),
            Ownership::ReapingOnly => self.reap_exact(),
            Ownership::Owned => unreachable!("signaling ended before exact reaping"),
        }
    }

    fn reap_exact(&mut self) -> io::Result<()> {
        loop {
            match reap::finish(
                self.first_failure.take(),
                || self.child.try_wait(),
                std::thread::yield_now,
            ) {
                Ok(reaped) => {
                    self.ownership = Ownership::Reaped;
                    (self.reaped)(reaped.status);
                    return reaped.failure.map_or(Ok(()), Err);
                }
                Err(error) if error.raw_os_error() == Some(libc::ECHILD) => {
                    self.ownership = Ownership::Lost;
                    return Err(error);
                }
                Err(error) => {
                    self.first_failure.get_or_insert(error);
                    std::thread::yield_now();
                }
            }
        }
    }
}

impl<R, F: Fn(ExitStatus)> Drop for OwnedGroup<R, F> {
    fn drop(&mut self) {
        let _ = self.finish(Instant::now, std::thread::sleep);
    }
}

pub(super) fn launch<O, R, F>(
    mut command: std::process::Command,
    now: impl Fn() -> Instant,
    wait: impl Fn(Duration),
    grace: Duration,
    observed: O,
    reaped: F,
) -> Result<(OwnedGroup<R, F>, PipeSlots, Instant), SpawnError>
where
    O: FnOnce(SignalLease, &mut PipeSlots, &mut Option<R>) -> io::Result<()>,
    F: Fn(ExitStatus),
{
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    command
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let program = command.get_program().to_string_lossy().into_owned();
    let started = now();
    let child = command.spawn().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            SpawnError::NotFound(program)
        } else {
            safe_error(error)
        }
    })?;
    let mut owned = OwnedGroup::new(child, grace, reaped);
    let pipes = match owned.register(observed) {
        Ok(pipes) => pipes,
        Err(error) => {
            if let Err(cleanup) = owned.finish(now, wait)
                && cleanup.raw_os_error() == Some(libc::ECHILD)
            {
                return Err(safe_error(cleanup));
            }
            return Err(safe_error(error));
        }
    };
    Ok((owned, pipes, started))
}
