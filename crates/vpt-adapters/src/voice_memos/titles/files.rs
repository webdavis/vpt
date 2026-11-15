use super::SIDE_FILES;
use crate::{Access, ContainedError, RootDir};
use std::fs::File;
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) fn acquire(
    root: &RootDir,
    name: &str,
    access: Access,
    create: bool,
) -> Result<File, ContainedError> {
    root.revalidate()?;
    let file = match root.open_file(Path::new(name), access) {
        Ok(file) => file,
        Err(ContainedError::Io {
            kind: ErrorKind::NotFound,
            ..
        }) if create => match root.create_read_write_file(Path::new(name), 0o600) {
            Ok(file) => file,
            Err(ContainedError::Io {
                kind: ErrorKind::AlreadyExists,
                ..
            }) => root.open_file(Path::new(name), access)?,
            Err(error) => return Err(error),
        },
        Err(error) => return Err(error),
    };
    verify(root, name, &file)?;
    Ok(file)
}

pub(super) fn verify(root: &RootDir, name: &str, file: &File) -> Result<(), ContainedError> {
    root.revalidate()?;
    let path = root.path().join(name);
    let escaped = || ContainedError::Escape {
        root: root.path().into(),
        path: path.clone(),
    };
    let owned = file.metadata().map_err(|error| ContainedError::Io {
        path: path.clone(),
        kind: error.kind(),
    })?;
    if !owned.is_file() {
        return Err(ContainedError::NotRegular(path));
    }
    if owned.nlink() != 1 {
        return Err(escaped());
    }
    match root.single_file(Path::new(name)) {
        Ok(Some(named)) if named.identity == (owned.dev(), owned.ino()) => {}
        Ok(_) | Err(ContainedError::NotRegular(_) | ContainedError::Escape { .. }) => {
            return Err(escaped());
        }
        Err(error) => return Err(error),
    }
    root.revalidate()
}

/// Checks each copied file by name without opening it, so no descriptor close releases the
/// record locks SQLite holds on the copy.
pub(super) fn validate(root: &RootDir) -> Result<(), ContainedError> {
    root.revalidate()?;
    for name in SIDE_FILES {
        root.single_file(Path::new(name))?;
    }
    Ok(())
}
