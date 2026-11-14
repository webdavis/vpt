use super::kept::kept_note;
use crate::contained::{Access, ContainedError, RootDir};
use crate::stores::digest_open;
use std::path::Path;
use vpt_application::ports::DirtyPublication;
use vpt_domain::digest::Sha256Digest;

/// Names of the kept files in `directory` that hold a person's version. A kept file that holds
/// one of vpt's own versions of a pending publication is left for recovery to settle.
pub fn kept_files(
    directory: &Path,
    pending: &[DirtyPublication],
) -> Result<Vec<String>, ContainedError> {
    let root = match RootDir::open(directory) {
        Ok(root) => root,
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut kept = Vec::new();
    for name in root.names()? {
        let Some(note) = kept_note(&name) else {
            continue;
        };
        if !vpts_own(&root, &name, note, pending) {
            kept.push(name);
        }
    }
    Ok(kept)
}

/// A kept file or note that cannot be read never counts as vpt's own, so it is still reported.
fn vpts_own(root: &RootDir, name: &str, note: &str, pending: &[DirtyPublication]) -> bool {
    let Some(kept) = digest_of(root, name) else {
        return false;
    };
    let current = digest_of(root, note);
    pending.iter().any(|entry| {
        entry.target.file_name().and_then(|name| name.to_str()) == Some(note)
            && (kept == entry.intended
                || Some(kept) == entry.expected_previous
                || Some(kept) == current)
    })
}

fn digest_of(root: &RootDir, name: &str) -> Option<Sha256Digest> {
    let mut file = root.open_file(Path::new(name), Access::Read).ok()?;
    digest_open(&mut file).ok()
}
