mod clone;
mod descriptor;
mod directory;
mod error;
mod file;
mod link;
mod owned_directory;
mod private_log;
mod private_state;
mod rename;
mod volume;

pub use error::ContainedError;
use error::{escape, io};
pub use link::Placement;
pub use private_log::{
    AppendEffects, AppendFailure, AppendReceipt, AppendSync, EventLog, LogError, LogInspection,
    LogRefusal, TailRepair,
};
pub use private_state::PrivateStateError;
pub(crate) use private_state::effective_user;
pub use volume::RenameSupport;

use descriptor::open_relative;
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

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

fn c_string(path: &Path) -> Result<CString, ContainedError> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io(path, std::io::ErrorKind::InvalidInput.into()))
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
        Ok(Self {
            file: descriptor::open_directory(path)?,
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
