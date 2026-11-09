use crate::{Access, RootDir};
use std::{fs::File, io::Read, os::unix::fs::MetadataExt, path::Path};
use vpt_application::ports::StoreError;

pub(super) fn validate(
    root: &RootDir,
    path: &Path,
    owned: &File,
    bytes: &[u8],
) -> Result<(), StoreError> {
    let changed = || StoreError::Changed(root.path().join(path));
    let mut named = root.open_file(path, Access::Read).map_err(|_| changed())?;
    let (original, current) = (
        owned.metadata().map_err(|_| changed())?,
        named.metadata().map_err(|_| changed())?,
    );
    if current.nlink() != 1 || (original.dev(), original.ino()) != (current.dev(), current.ino()) {
        return Err(changed());
    }
    let mut buffer = [0; 8192];
    for chunk in bytes.chunks(buffer.len()) {
        named
            .read_exact(&mut buffer[..chunk.len()])
            .map_err(|_| changed())?;
        if &buffer[..chunk.len()] != chunk {
            return Err(changed());
        }
    }
    if named.read(&mut buffer[..1]).map_err(|_| changed())? != 0 {
        return Err(changed());
    }
    Ok(())
}
