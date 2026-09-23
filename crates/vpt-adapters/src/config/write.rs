mod directory;

use crate::{Access, ContainedError};
use directory::PlannedDirectory;
use std::fs::Permissions;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use vpt_application::{SetupError, SetupWriter};

pub struct FilesystemSetupWriter {
    config_path: PathBuf,
    config_parent: PlannedDirectory,
    directories: Vec<PlannedDirectory>,
}

impl FilesystemSetupWriter {
    pub fn new(config_path: PathBuf, directories: &[PathBuf]) -> Result<Self, SetupError> {
        let parent = config_path
            .parent()
            .ok_or_else(|| SetupError::PathEscape(config_path.clone()))?;
        Ok(Self {
            config_parent: PlannedDirectory::capture_config(parent)?,
            config_path,
            directories: directories
                .iter()
                .map(|path| PlannedDirectory::capture(path))
                .collect::<Result<_, _>>()?,
        })
    }
}

fn io(error: std::io::Error) -> SetupError {
    SetupError::Io(error.kind().to_string())
}

fn contained(error: ContainedError) -> SetupError {
    match error {
        ContainedError::Io {
            path,
            kind: std::io::ErrorKind::AlreadyExists,
        } => SetupError::Exists(path),
        ContainedError::Io { kind, .. } => SetupError::Io(kind.to_string()),
        ContainedError::Escape { path, .. }
        | ContainedError::NotRegular(path)
        | ContainedError::NotADirectory(path) => SetupError::PathEscape(path),
    }
}

impl SetupWriter for FilesystemSetupWriter {
    fn config_path(&self) -> PathBuf {
        self.config_path.clone()
    }
    fn config_exists(&self) -> bool {
        self.config_path.symlink_metadata().is_ok()
    }

    fn write_config(&self, text: &str, force: bool) -> Result<PathBuf, SetupError> {
        let root = self.config_parent.create_config()?;
        root.revalidate().map_err(contained)?;
        let name = self
            .config_path
            .file_name()
            .ok_or_else(|| SetupError::PathEscape(self.config_path.clone()))?;
        let name = Path::new(name);
        let mut file = if force {
            match root.open_file(name, Access::Write) {
                Ok(file) => file,
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => root.create_file(name, 0o600).map_err(contained)?,
                Err(error) => return Err(contained(error)),
            }
        } else {
            root.create_file(name, 0o600).map_err(contained)?
        };
        if file.metadata().map_err(io)?.nlink() > 1 {
            return Err(SetupError::MultiplyLinkedConfig(self.config_path.clone()));
        }
        file.set_permissions(Permissions::from_mode(0o600))
            .map_err(io)?;
        if force {
            file.set_len(0).map_err(io)?;
        }
        file.write_all(text.as_bytes()).map_err(io)?;
        file.sync_all().map_err(io)?;
        root.revalidate().map_err(contained)?;
        root.sync().map_err(contained)?;
        Ok(self.config_path.clone())
    }

    fn create_private_dir(&self, path: &Path) -> Result<(), SetupError> {
        self.directories
            .iter()
            .find(|directory| directory.path == path)
            .ok_or_else(|| SetupError::PathEscape(path.into()))?
            .create()
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests;
