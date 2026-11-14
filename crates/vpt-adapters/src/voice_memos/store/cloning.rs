use crate::{ContainedError, RootDir};
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::fs::FileExt;
use std::path::Path;
use vpt_application::ports::{CloneError, CloneKind};

const BUFFER: usize = 64 * 1024;

pub(super) fn clone_into(
    source: &File,
    directory: &Path,
    identity: (u64, u64),
    name: &str,
) -> Result<CloneKind, CloneError> {
    clone_into_with(source, directory, identity, name, native_clone)
}

pub(super) fn clone_into_with(
    source: &File,
    directory: &Path,
    identity: (u64, u64),
    name: &str,
    clone: impl FnOnce(&File, &RootDir, &Path) -> io::Result<()>,
) -> Result<CloneKind, CloneError> {
    let root = RootDir::open(directory).map_err(|error| path_error(directory, error))?;
    if root
        .identity()
        .map_err(|error| path_error(directory, error))?
        != identity
    {
        return Err(CloneError::Escape(directory.into()));
    }
    let target = root
        .leaf(Path::new(name))
        .map_err(|error| path_error(&directory.join(name), error))?;
    match clone(source, &root, &target) {
        Ok(()) => Ok(CloneKind::CopyOnWrite),
        Err(error) if error.raw_os_error() == Some(libc::EXDEV) => {
            byte_copy(source, &root, Path::new(name))
        }
        Err(error) => Err(acquisition_error(error)),
    }
}

fn native_clone(source: &File, directory: &RootDir, leaf: &Path) -> io::Result<()> {
    directory.clone_file(source, leaf)
}

pub(super) fn byte_copy(
    source: &File,
    directory: &RootDir,
    name: &Path,
) -> Result<CloneKind, CloneError> {
    let mut out = directory
        .create_file(name, 0o600)
        .map_err(|error| path_error(&directory.path().join(name), error))?;
    copy_contents(source, &mut out)?;
    Ok(CloneKind::ByteCopy)
}

pub(super) fn copy_contents(source: &File, out: &mut impl Write) -> Result<(), CloneError> {
    let total = source.metadata().map_err(io_error)?.len();
    let mut buffer = [0; BUFFER];
    let mut offset = 0;
    while offset < total {
        let chunk = usize::try_from((total - offset).min(BUFFER as u64)).unwrap_or(BUFFER);
        source
            .read_exact_at(&mut buffer[..chunk], offset)
            .map_err(io_error)?;
        out.write_all(&buffer[..chunk]).map_err(write_error)?;
        offset += chunk as u64;
    }
    Ok(())
}

fn io_error(error: io::Error) -> CloneError {
    CloneError::Io(error.kind().to_string())
}

fn write_error(error: io::Error) -> CloneError {
    if error.kind() == io::ErrorKind::StorageFull {
        CloneError::NoSpace
    } else {
        io_error(error)
    }
}

fn acquisition_error(error: io::Error) -> CloneError {
    match error.kind() {
        io::ErrorKind::StorageFull => CloneError::NoSpace,
        io::ErrorKind::AlreadyExists => CloneError::Exists,
        kind => CloneError::Io(kind.to_string()),
    }
}

pub(super) fn path_error(path: &Path, error: ContainedError) -> CloneError {
    match error {
        ContainedError::Escape { .. }
        | ContainedError::NotRegular(_)
        | ContainedError::NotADirectory(_) => CloneError::Escape(path.into()),
        ContainedError::Io { kind, .. } => acquisition_error(kind.into()),
    }
}
