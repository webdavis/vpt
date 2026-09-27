use crate::contained::{Access, ContainedError, RootDir};
use std::fs::File;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::{Duration, Instant};

const LOCK_FILE: &str = "write.lock";
const POLL: Duration = Duration::from_millis(25);

#[derive(Debug, PartialEq, Eq)]
pub enum LockError {
    Busy,
    Contained(ContainedError),
    Io(String),
}

pub struct WriteLock {
    file: File,
}

impl WriteLock {
    pub fn acquire(state: &RootDir, wait: Duration) -> Result<Self, LockError> {
        let file = open_lock_file(state).map_err(LockError::Contained)?;
        let started = Instant::now();
        if try_lock(file.as_raw_fd())? {
            return Ok(Self { file });
        }
        while started.elapsed() < wait {
            std::thread::sleep(POLL.min(wait.saturating_sub(started.elapsed())));
            if started.elapsed() >= wait {
                break;
            }
            if try_lock(file.as_raw_fd())? {
                if started.elapsed() >= wait {
                    break;
                }
                return Ok(Self { file });
            }
        }
        Err(LockError::Busy)
    }
}

fn open_lock_file(state: &RootDir) -> Result<File, ContainedError> {
    let name = Path::new(LOCK_FILE);
    let file = match state.open_file(name, Access::ReadWrite) {
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => match state.create_read_write_file(name, 0o600) {
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::AlreadyExists,
                ..
            }) => state.open_file(name, Access::ReadWrite),
            created => created,
        },
        opened => opened,
    }?;
    let path = state.path().join(LOCK_FILE);
    let metadata = file.metadata().map_err(|error| ContainedError::Io {
        path: path.clone(),
        kind: error.kind(),
    })?;
    if metadata.nlink() != 1 {
        return Err(ContainedError::Escape {
            root: state.path().into(),
            path,
        });
    }
    Ok(file)
}

fn try_lock(fd: RawFd) -> Result<bool, LockError> {
    // SAFETY: flock takes no pointer; LOCK_NB makes this attempt nonblocking.
    if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::EWOULDBLOCK) {
        Ok(false)
    } else {
        Err(LockError::Io(error.to_string()))
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        // SAFETY: this value owns the live lock descriptor until drop completes.
        unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[cfg(test)]
pub(crate) mod tests;
