use super::{FilesystemStores, store_error};
use crate::contained::{ContainedError, Placement, RenameSupport, RootDir};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use vpt_application::ports::StoreError;

const MARK: &str = ".vpt-kept-";

pub(super) fn kept_note(name: &str) -> Option<&str> {
    let (note, suffix) = name.strip_prefix('.')?.rsplit_once(MARK)?;
    let (process, sequence) = suffix.split_once('-')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    (!note.is_empty() && digits(process) && digits(sequence)).then_some(note)
}

impl FilesystemStores {
    pub(in crate::stores) fn kept_files_beside(
        &self,
        note: &Path,
    ) -> Result<Vec<PathBuf>, StoreError> {
        let (root, leaf) = self.owner(note)?;
        let names = root.names().map_err(|error| store_error(note, error))?;
        Ok(names
            .into_iter()
            .filter(|name| kept_note(name) == leaf.file_name().and_then(|name| name.to_str()))
            .map(|name| root.path().join(name))
            .collect())
    }

    pub(in crate::stores) fn return_kept_file(
        &self,
        kept: &Path,
        note: &Path,
    ) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(note)?;
        let support = root.rename_support();
        if !support.places_exclusively() {
            return Err(StoreError::UnsafeVolume(note.to_path_buf()));
        }
        match super::place::place_exclusive(root, support, kept, &leaf, &mut |_| Ok(()))? {
            Placement::Placed => Ok(()),
            Placement::Occupied => self.remove_kept_file(kept),
        }
    }

    pub(in crate::stores) fn remove_kept_file(&self, kept: &Path) -> Result<(), StoreError> {
        let (root, leaf) = self.owner(kept)?;
        root.unlink_unchecked(&leaf)
            .map_err(|error| store_error(kept, error))
    }

    /// Returns `None` when `from` no longer exists.
    pub(super) fn set_aside(
        &self,
        root: &RootDir,
        support: RenameSupport,
        from: &Path,
        note: &str,
    ) -> Result<Option<PathBuf>, StoreError> {
        for _ in 0..64 {
            let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
            let name = format!(".{note}{MARK}{}-{sequence}", std::process::id());
            let kept = root.path().join(name);
            match move_to_free_name(root, support, from, &kept) {
                Ok(Placement::Placed) => return Ok(Some(kept)),
                Ok(Placement::Occupied) => {}
                Err(ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                }) => return Ok(None),
                Err(error) => return Err(store_error(&kept, error)),
            }
        }
        Err(StoreError::Io("publication kept names exhausted".into()))
    }
}

/// Without an exclusive rename, `from` replaces only an empty placeholder vpt has just created.
fn move_to_free_name(
    root: &RootDir,
    support: RenameSupport,
    from: &Path,
    kept: &Path,
) -> Result<Placement, ContainedError> {
    if support.exclusive {
        return Ok(if root.rename_exclusive(from, kept)? {
            Placement::Placed
        } else {
            Placement::Occupied
        });
    }
    match root.create_file(kept, 0o600) {
        Ok(_) => {}
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::AlreadyExists,
            ..
        }) => return Ok(Placement::Occupied),
        Err(error) => return Err(error),
    }
    if let Err(error) = root.rename_over(from, kept) {
        root.unlink_file(kept)?;
        return Err(error);
    }
    Ok(Placement::Placed)
}
