use super::{contained, io};
use crate::{ContainedError, RootDir};
use std::ffi::OsString;
use std::fs::{File, Permissions};
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use vpt_application::SetupError;

pub(super) struct PlannedDirectory {
    pub path: PathBuf,
    anchor: RootDir,
    missing: Vec<OsString>,
}

impl PlannedDirectory {
    pub fn capture(path: &Path) -> Result<Self, SetupError> {
        if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(SetupError::PathEscape(path.into()));
        }
        let mut cursor = path;
        let mut missing = Vec::new();
        loop {
            match RootDir::open(cursor) {
                Ok(anchor) => {
                    missing.reverse();
                    return Ok(Self {
                        path: path.into(),
                        anchor,
                        missing,
                    });
                }
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => {
                    missing.push(
                        cursor
                            .file_name()
                            .ok_or_else(|| SetupError::PathEscape(path.into()))?
                            .to_os_string(),
                    );
                    cursor = cursor
                        .parent()
                        .ok_or_else(|| SetupError::PathEscape(path.into()))?;
                }
                Err(error) => return Err(contained(error)),
            }
        }
    }

    pub fn create(&self) -> Result<RootDir, SetupError> {
        let directory = self.walk()?;
        let file = File::from(directory.as_fd().try_clone_to_owned().map_err(io)?);
        file.set_permissions(Permissions::from_mode(0o700))
            .map_err(io)?;
        Ok(directory)
    }

    pub fn capture_config(path: &Path) -> Result<Self, SetupError> {
        let planned = Self::capture(path)?;
        if planned.missing.is_empty() {
            planned.check_private(&planned.anchor)?;
        }
        Ok(planned)
    }

    pub fn create_config(&self) -> Result<RootDir, SetupError> {
        let directory = self.walk()?;
        self.check_private(&directory)?;
        Ok(directory)
    }

    fn check_private(&self, directory: &RootDir) -> Result<(), SetupError> {
        directory.revalidate().map_err(contained)?;
        let file = File::from(directory.as_fd().try_clone_to_owned().map_err(io)?);
        if file.metadata().map_err(io)?.permissions().mode() & 0o777 != 0o700 {
            return Err(SetupError::NonPrivateConfig(self.path.clone()));
        }
        Ok(())
    }

    fn walk(&self) -> Result<RootDir, SetupError> {
        self.anchor.revalidate().map_err(contained)?;
        let mut directory = self.anchor.try_clone().map_err(contained)?;
        for component in &self.missing {
            directory.revalidate().map_err(contained)?;
            let component = component
                .to_str()
                .ok_or_else(|| SetupError::Io("directory name is not UTF-8".into()))?;
            directory = directory.subdirectory(component).map_err(contained)?;
        }
        directory.revalidate().map_err(contained)?;
        Ok(directory)
    }
}
