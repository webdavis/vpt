use super::{Status, membership, process};
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub(super) struct SignalLease {
    pid: libc::pid_t,
    retained: Arc<Mutex<bool>>,
}

struct Permit<'a> {
    pid: libc::pid_t,
    retained: MutexGuard<'a, bool>,
}

impl SignalLease {
    pub(super) fn new(pid: libc::pid_t) -> Self {
        Self {
            pid,
            retained: Arc::new(Mutex::new(true)),
        }
    }

    fn enter(&self) -> io::Result<Permit<'_>> {
        let retained = self.lock();
        if !*retained {
            return Err(io::Error::from_raw_os_error(libc::ECHILD));
        }
        Ok(Permit {
            pid: self.pid,
            retained,
        })
    }

    pub(super) fn revoke(&self) {
        *self.lock() = false;
    }

    pub(super) fn group(&self) -> libc::pid_t {
        self.pid
    }

    pub(super) fn observe(&self) -> io::Result<Option<Status>> {
        let mut permit = self.enter()?;
        let result = process::observe(permit.pid);
        permit.record(result)
    }

    pub(super) fn signal(&self, signal: libc::c_int) -> io::Result<()> {
        let permit = self.enter()?;
        // SAFETY: the exclusive permit retains the owned unreaped group leader.
        if unsafe { libc::killpg(permit.pid, signal) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(super) fn only_terminal_leader(&self) -> io::Result<bool> {
        let mut permit = self.enter()?;
        let result = process::observe(permit.pid);
        let terminal = permit.record(result)?;
        if terminal.is_none() {
            return Ok(false);
        }
        let mut members = [0; 2];
        // SAFETY: the fixed writable buffer belongs to this exact retained group query.
        let count = unsafe {
            libc::proc_listpgrppids(
                permit.pid,
                members.as_mut_ptr().cast(),
                std::mem::size_of_val(&members) as libc::c_int,
            )
        };
        Ok(membership::only_terminal_leader(
            permit.pid, terminal, count, &members,
        ))
    }

    fn lock(&self) -> MutexGuard<'_, bool> {
        self.retained
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Permit<'_> {
    fn record(&mut self, result: io::Result<Option<Status>>) -> io::Result<Option<Status>> {
        if result
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(libc::ECHILD))
        {
            *self.retained = false;
        }
        result
    }
}

#[cfg(test)]
mod tests;
