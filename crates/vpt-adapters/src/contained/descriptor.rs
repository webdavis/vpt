use super::{ContainedError, c_string, io};
use std::fs::File;
use std::os::fd::FromRawFd;
use std::path::Path;

pub(super) fn opened(
    fd: libc::c_int,
    path: &Path,
    directory: bool,
) -> Result<File, ContainedError> {
    if fd >= 0 {
        // SAFETY: each caller passes a fresh successful open/openat result, transferring ownership.
        return Ok(unsafe { File::from_raw_fd(fd) });
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::ELOOP | libc::ENOTDIR) if directory => {
            Err(ContainedError::NotADirectory(path.into()))
        }
        Some(libc::ELOOP | libc::EISDIR | libc::ENXIO) if !directory => {
            Err(ContainedError::NotRegular(path.into()))
        }
        _ => Err(io(path, error)),
    }
}

pub(super) fn open_directory(path: &Path) -> Result<File, ContainedError> {
    let name = c_string(path)?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY | libc::O_CLOEXEC;
    // SAFETY: name is NUL-terminated and live for the call; these flags need no mode argument.
    let fd = unsafe { libc::open(name.as_ptr(), flags) };
    opened(fd, path, true)
}
