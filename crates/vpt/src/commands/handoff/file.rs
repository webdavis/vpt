use super::errors;
use std::path::Path;
use vpt_adapters::{ContainedError, FilesystemStores};
use vpt_application::ports::{NoteFiles, StoreError, StoredNote};
use vpt_domain::digest::Sha256Digest;
use vpt_protocol::error::ErrorDocument;
pub(super) fn read(path: &Path, root: &Path) -> Result<Option<StoredNote>, Box<ErrorDocument>> {
    read_inner(path, root, None)
}
pub(super) fn version(
    path: &Path,
    root: &Path,
    expected: Sha256Digest,
) -> Result<Option<StoredNote>, Box<ErrorDocument>> {
    read_inner(path, root, Some(expected))
}
fn read_inner(
    path: &Path,
    root: &Path,
    expected: Option<Sha256Digest>,
) -> Result<Option<StoredNote>, Box<ErrorDocument>> {
    if path.parent() != Some(root) {
        return Err(errors::changed());
    }
    let stores = match FilesystemStores::open(&[root.to_path_buf()]) {
        Ok(stores) => stores,
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => return Ok(None),
        Err(error) => return Err(Box::new(crate::compose::errors::contained_error(error))),
    };
    match expected {
        Some(digest) => stores.read_note_version(path, digest),
        None => stores.read_note(path),
    }
    .map_err(|e| match e {
        StoreError::Changed(_) => errors::changed(),
        other => Box::new(crate::compose::errors::store_error(other)),
    })
}
pub(super) fn body(text: &str) -> Result<&str, Box<ErrorDocument>> {
    vpt_adapters::note_document::body(text).map_err(|_| errors::changed())
}

pub(super) fn exists(path: &Path) -> Result<bool, Box<ErrorDocument>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(Box::new(vpt_protocol::error::ErrorDocument::new(
            vpt_protocol::error::ErrorKind::Store,
            "cannot inspect handoff artifact",
        ))),
    }
}
