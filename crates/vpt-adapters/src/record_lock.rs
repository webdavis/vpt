use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;

pub(crate) enum LockKind {
    Shared,
    Exclusive,
    #[cfg(test)]
    Unlock,
}

/// Sets an open-file-description lock without waiting, so closing another descriptor for the same
/// file leaves it in place. Contention returns `WouldBlock`.
pub(crate) fn set_record_lock(
    file: &File,
    start: i64,
    length: i64,
    kind: LockKind,
) -> io::Result<()> {
    if start < 0 || length < 0 || start.checked_add(length).is_none() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    // SAFETY: flock is an integer-only C structure; all-zero fields are valid.
    let mut lock: libc::flock = unsafe { std::mem::zeroed() };
    lock.l_whence = libc::SEEK_SET as libc::c_short;
    lock.l_start = start;
    lock.l_len = length;
    lock.l_type = match kind {
        LockKind::Shared => libc::F_RDLCK,
        LockKind::Exclusive => libc::F_WRLCK,
        #[cfg(test)]
        LockKind::Unlock => libc::F_UNLCK,
    };
    loop {
        // SAFETY: the owned file and initialized flock remain live for this nonblocking call.
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_OFD_SETLK, &mut lock) } != -1 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::EINTR) => continue,
            Some(libc::EACCES | libc::EAGAIN) => return Err(io::ErrorKind::WouldBlock.into()),
            _ => return Err(error),
        }
    }
}
