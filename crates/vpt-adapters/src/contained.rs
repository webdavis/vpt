mod directory;
mod file;
mod private_log;
mod private_state;
mod rename;

pub use private_log::{
    AppendEffects, AppendFailure, AppendReceipt, AppendSync, EventLog, LogError, LogInspection,
    LogRefusal, TailRepair,
};
pub use private_state::PrivateStateError;
pub(crate) use private_state::effective_user;

use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainedError {
    Escape {
        root: PathBuf,
        path: PathBuf,
    },
    NotRegular(PathBuf),
    NotADirectory(PathBuf),
    Io {
        path: PathBuf,
        kind: std::io::ErrorKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Directory,
    Link,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stat {
    pub kind: Kind,
    pub size: u64,
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
    pub flags: u32,
}

#[derive(Debug)]
pub struct RootDir {
    file: File,
    path: PathBuf,
}

const NO_FOLLOW: libc::c_int = libc::O_NOFOLLOW | libc::O_CLOEXEC;

fn escape(root: &Path, path: &Path) -> ContainedError {
    ContainedError::Escape {
        root: root.into(),
        path: path.into(),
    }
}

fn io(path: &Path, error: std::io::Error) -> ContainedError {
    ContainedError::Io {
        path: path.into(),
        kind: error.kind(),
    }
}

fn c_string(path: &Path) -> Result<CString, ContainedError> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io(path, std::io::ErrorKind::InvalidInput.into()))
}

fn opened(fd: libc::c_int, path: &Path, directory: bool) -> Result<File, ContainedError> {
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

pub fn leaf_below(root: &Path, path: &Path) -> Result<PathBuf, ContainedError> {
    let relative = if path.is_absolute() {
        path.strip_prefix(root).map_err(|_| escape(root, path))?
    } else {
        path
    };
    let mut components = relative.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(name)), None) => Ok(root.join(name)),
        _ => Err(escape(root, path)),
    }
}

impl RootDir {
    pub fn open(path: &Path) -> Result<Self, ContainedError> {
        let name = c_string(path)?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW_ANY | libc::O_CLOEXEC;
        // SAFETY: name is NUL-terminated and live for the call; these flags need no mode argument.
        let fd = unsafe { libc::open(name.as_ptr(), flags) };
        Ok(Self {
            file: opened(fd, path, true)?,
            path: path.into(),
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn try_clone(&self) -> Result<Self, ContainedError> {
        Ok(Self {
            file: self
                .file
                .try_clone()
                .map_err(|error| io(&self.path, error))?,
            path: self.path.clone(),
        })
    }
    pub fn identity(&self) -> Result<(u64, u64), ContainedError> {
        let metadata = self
            .file
            .metadata()
            .map_err(|error| io(&self.path, error))?;
        Ok((metadata.dev(), metadata.ino()))
    }
    pub fn revalidate(&self) -> Result<(), ContainedError> {
        let current = Self::open(&self.path).map_err(|error| match error {
            ContainedError::NotADirectory(_)
            | ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            } => escape(&self.path, &self.path),
            error => error,
        })?;
        if self.identity()? != current.identity()? {
            return Err(escape(&self.path, &self.path));
        }
        Ok(())
    }
    pub fn leaf(&self, path: &Path) -> Result<PathBuf, ContainedError> {
        leaf_below(&self.path, path)
    }
    pub fn c_name(&self, path: &Path) -> Result<CString, ContainedError> {
        let leaf = self.leaf(path)?;
        let name = leaf.file_name().ok_or_else(|| escape(&self.path, path))?;
        c_string(Path::new(name))
    }
    pub fn sync(&self) -> Result<(), ContainedError> {
        self.file.sync_all().map_err(|error| io(&self.path, error))
    }
}

impl AsFd for RootDir {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

#[cfg(test)]
mod tests;
