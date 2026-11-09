use crate::{Access, RootDir};
use std::fs::{File, Metadata, Permissions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use vpt_application::ports::EngineFailure;

pub(super) fn copy(
    input: File,
    source_root: &RootDir,
    source: &Path,
    target: &RootDir,
    maximum: u64,
) -> Result<(), EngineFailure> {
    let before = input.metadata().map_err(|_| EngineFailure::Io)?;
    if before.len() > maximum {
        return Err(EngineFailure::Io);
    }
    let mut reader = input.take(before.len().saturating_add(1));
    let mut output = target
        .create_file(Path::new("audio.m4a"), 0o600)
        .map_err(|_| EngineFailure::Io)?;
    let copied = std::io::copy(&mut reader, &mut output).map_err(|_| EngineFailure::Io)?;
    let after = reader.get_ref().metadata().map_err(|_| EngineFailure::Io)?;
    let named = source_root
        .open_file(source, Access::Read)
        .map_err(|_| EngineFailure::Io)?;
    let named = named.metadata().map_err(|_| EngineFailure::Io)?;
    if copied != before.len()
        || identity(&before) != identity(&after)
        || identity(&after) != identity(&named)
    {
        return Err(EngineFailure::Io);
    }
    source_root.revalidate().map_err(|_| EngineFailure::Io)?;
    target.revalidate().map_err(|_| EngineFailure::Io)
}

fn identity(metadata: &Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

pub(super) fn read_document(root: &RootDir) -> Result<Vec<u8>, EngineFailure> {
    let file = root
        .open_file(Path::new("audio.json"), Access::Read)
        .map_err(|_| EngineFailure::Io)?;
    let metadata = file.metadata().map_err(|_| EngineFailure::Io)?;
    if metadata.nlink() != 1 {
        return Err(EngineFailure::Io);
    }
    let limit = super::super::DOCUMENT_LIMIT;
    if metadata.len() > limit as u64 {
        return Err(EngineFailure::OutputLimit);
    }
    file.set_permissions(Permissions::from_mode(0o600))
        .map_err(|_| EngineFailure::Io)?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| EngineFailure::Io)?;
    if bytes.len() > limit {
        return Err(EngineFailure::OutputLimit);
    }
    Ok(bytes)
}
