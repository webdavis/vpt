use super::Status;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, OwnedFd};

pub(super) fn observe(pid: libc::pid_t) -> io::Result<Option<Status>> {
    // SAFETY: siginfo_t is an output structure initialized before waitid writes it.
    let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
    // SAFETY: the exact direct child remains waitable until cleanup releases its identity.
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
    decode_status(info.si_code, info.si_status).map(Some)
}

pub(super) fn pipe(slot: Option<OwnedFd>) -> io::Result<File> {
    let file = File::from(slot.ok_or(io::ErrorKind::InvalidInput)?);
    // SAFETY: file retains this live descriptor throughout both flag operations.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: only status flags of the same retained descriptor change.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

fn decode_status(code: libc::c_int, status: libc::c_int) -> io::Result<Status> {
    match code {
        libc::CLD_EXITED => Ok(Status::Exited(status)),
        libc::CLD_KILLED | libc::CLD_DUMPED => Ok(Status::Signaled(status)),
        _ => Err(io::ErrorKind::InvalidData.into()),
    }
}

#[cfg(test)]
mod tests;
