use super::{ContainedError, c_string, io};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::path::Path;

pub(super) fn open_relative(
    directory: BorrowedFd<'_>,
    name: &CStr,
    flags: libc::c_int,
    mode: u32,
    path: &Path,
) -> Result<File, ContainedError> {
    open_at(directory, name, flags, mode).map_err(|error| refused(path, flags, error))
}

pub(super) fn open_at(
    directory: BorrowedFd<'_>,
    name: &CStr,
    flags: libc::c_int,
    mode: u32,
) -> std::io::Result<File> {
    // SAFETY: directory is a live borrowed descriptor and name is NUL-terminated; mode is promoted.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags,
            mode as libc::c_uint,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: fd is the fresh descriptor this successful openat returned; nothing else owns it.
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(fd) }))
}

pub(super) fn open_directory(path: &Path) -> Result<File, ContainedError> {
    let name = c_string(path)?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY | libc::O_CLOEXEC;
    // SAFETY: name is NUL-terminated and live for the call; these flags need no mode argument.
    let fd = unsafe { libc::open(name.as_ptr(), flags) };
    if fd < 0 {
        return Err(refused(path, flags, std::io::Error::last_os_error()));
    }
    // SAFETY: fd is the fresh descriptor this successful open returned; nothing else owns it.
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(fd) }))
}

fn refused(path: &Path, flags: libc::c_int, error: std::io::Error) -> ContainedError {
    match error.raw_os_error() {
        Some(libc::ELOOP | libc::ENOTDIR) if flags & libc::O_DIRECTORY != 0 => {
            ContainedError::NotADirectory(path.into())
        }
        Some(libc::ELOOP | libc::EISDIR | libc::ENXIO) if flags & libc::O_DIRECTORY == 0 => {
            ContainedError::NotRegular(path.into())
        }
        _ => io(path, error),
    }
}
