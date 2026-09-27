use super::{COPY_DIRECTORY, DATABASE, SIDE_FILES, files, lease::CopyLease};
use crate::database::DatabaseError;
use crate::{Access, ContainedError, RootDir};
use std::fs::{File, Permissions};
use std::io::{ErrorKind, Seek, SeekFrom};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

pub(super) fn refresh(
    container: &Path,
    state: &RootDir,
    timeout: Duration,
) -> Result<CopyLease, DatabaseError> {
    state.revalidate()?;
    let source = RootDir::open(container)?;
    let main = source.open_file(Path::new(DATABASE), Access::Read)?;
    let wal = companion(&source, SIDE_FILES[1])?;
    let shm = companion(&source, SIDE_FILES[2])?;
    let copy = state.subdirectory(COPY_DIRECTORY)?;
    let lease = CopyLease::refresh(&copy, timeout)?;
    files::validate(&copy)?;
    let copied_wal = files::acquire(&copy, SIDE_FILES[1], Access::ReadWrite, true)?;
    let copied_shm = files::acquire(&copy, SIDE_FILES[2], Access::ReadWrite, true)?;
    for ((name, input), output) in SIDE_FILES.into_iter().zip([Some(main), wal, shm]).zip([
        lease.file(),
        &copied_wal,
        &copied_shm,
    ]) {
        source.revalidate()?;
        lease.revalidate()?;
        files::verify(&copy, name, output)?;
        overwrite(output, input.as_ref())?;
    }
    copy.revalidate()?;
    Ok(lease)
}

fn companion(source: &RootDir, name: &str) -> Result<Option<File>, ContainedError> {
    match source.open_file(Path::new(name), Access::Read) {
        Ok(file) => Ok(Some(file)),
        Err(ContainedError::Io {
            kind: ErrorKind::NotFound,
            ..
        }) => Ok(None),
        Err(error) => Err(error),
    }
}

fn overwrite(mut target: &File, source: Option<&File>) -> Result<(), DatabaseError> {
    let failed = |error: std::io::Error| DatabaseError::Io {
        operation: "title copy",
        kind: error.kind(),
    };
    target
        .set_permissions(Permissions::from_mode(0o600))
        .map_err(failed)?;
    target.set_len(0).map_err(failed)?;
    target.seek(SeekFrom::Start(0)).map_err(failed)?;
    if let Some(mut source) = source {
        std::io::copy(&mut source, &mut target).map_err(failed)?;
    }
    Ok(())
}
