use super::{AccessPolicy, DatabaseError, FileRole};
use crate::{Access, ContainedError, RootDir};
use std::ffi::{CString, OsStr, OsString};
use std::fs::{File, Permissions};
use std::io::ErrorKind;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

pub(super) struct DatabaseFiles {
    root: RootDir,
    leaf: CString,
    policy: AccessPolicy,
    pub(super) faults: super::faults::FaultState,
}

impl DatabaseFiles {
    pub(super) fn new(
        root: RootDir,
        leaf: &Path,
        policy: AccessPolicy,
    ) -> Result<Self, DatabaseError> {
        let invalid = || ContainedError::Escape {
            root: root.path().into(),
            path: leaf.into(),
        };
        if leaf.file_name() != Some(leaf.as_os_str()) {
            return Err(invalid().into());
        }
        let leaf = CString::new(leaf.as_os_str().as_bytes()).map_err(|_| invalid())?;
        Ok(Self {
            root,
            leaf,
            policy,
            faults: Default::default(),
        })
    }

    fn name(&self, role: FileRole) -> OsString {
        let mut name = OsStr::from_bytes(self.leaf.as_bytes()).to_os_string();
        name.push(match role {
            FileRole::Database => "",
            FileRole::Journal => "-journal",
            FileRole::Wal => "-wal",
            FileRole::SharedMemory => "-shm",
        });
        name
    }

    pub(super) fn open(&self, role: FileRole, create: bool) -> Result<File, DatabaseError> {
        let name = self.name(role);
        let path = Path::new(&name);
        let writable = self.policy.writable(role);
        if writable && create {
            match self.create(role) {
                Ok(file) => return Ok(file),
                Err(DatabaseError::Contained(ContainedError::Io {
                    kind: ErrorKind::AlreadyExists,
                    ..
                })) => {}
                Err(error) => return Err(error),
            }
        }
        let access = if writable {
            Access::ReadWrite
        } else {
            Access::Read
        };
        let file = self.root.open_file(path, access)?;
        let metadata = file.metadata().map_err(|error| DatabaseError::Io {
            operation: "file metadata",
            kind: error.kind(),
        })?;
        if metadata.nlink() != 1 {
            return Err(ContainedError::Escape {
                root: self.root.path().into(),
                path: self.root.path().join(path),
            }
            .into());
        }
        if writable {
            file.set_permissions(Permissions::from_mode(0o600))
                .map_err(|error| DatabaseError::Io {
                    operation: "file permissions",
                    kind: error.kind(),
                })?;
        }
        Ok(file)
    }

    pub(super) fn create(&self, role: FileRole) -> Result<File, DatabaseError> {
        if !self.policy.writable(role) {
            return Err(super::sqlite_error(rusqlite::ffi::SQLITE_READONLY));
        }
        let file = self
            .root
            .create_read_write_file(Path::new(&self.name(role)), 0o600)?;
        self.sync()?;
        Ok(file)
    }

    pub(super) fn remove(&self, role: FileRole, sync: bool) -> Result<(), DatabaseError> {
        if !self.policy.writable(role) {
            return Err(DatabaseError::Io {
                operation: "remove",
                kind: ErrorKind::PermissionDenied,
            });
        }
        self.root.unlink_file(Path::new(&self.name(role)))?;
        if sync {
            self.sync()?;
        }
        Ok(())
    }

    pub(super) fn revalidate(&self) -> Result<(), DatabaseError> {
        self.root.revalidate().map_err(Into::into)
    }

    pub(super) fn sync(&self) -> Result<(), DatabaseError> {
        self.faults.check("directory sync")?;
        self.root.sync().map_err(Into::into)
    }

    pub(super) fn policy(&self) -> AccessPolicy {
        self.policy
    }

    pub(super) fn metadata(
        &self,
        role: FileRole,
        write: bool,
    ) -> Result<Option<std::fs::Metadata>, DatabaseError> {
        let file = match self.root.open_file(
            Path::new(&self.name(role)),
            if write {
                Access::ReadWrite
            } else {
                Access::Read
            },
        ) {
            Ok(file) => file,
            Err(ContainedError::Io {
                kind: ErrorKind::NotFound,
                ..
            }) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let metadata = file
            .metadata()
            .map_err(|error| super::io_error("file metadata", error))?;
        if metadata.nlink() != 1 {
            return Err(ContainedError::Escape {
                root: self.root.path().into(),
                path: self.root.path().join(self.name(role)),
            }
            .into());
        }
        Ok(Some(metadata))
    }

    pub(super) fn moved(&self, role: FileRole, file: &File) -> Result<bool, DatabaseError> {
        let Some(named) = self.metadata(role, false)? else {
            return Ok(true);
        };
        let opened = file
            .metadata()
            .map_err(|error| super::io_error("file metadata", error))?;
        Ok(named.dev() != opened.dev() || named.ino() != opened.ino())
    }
}
