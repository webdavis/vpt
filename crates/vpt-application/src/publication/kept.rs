use super::RepairError;
use crate::ports::{DirtyPublication, KeptEdit, KeptVersions, StoreError, Stores};

/// Finishes what a crash left of vpt's own versions beside the note without asking anyone, and
/// refuses while a kept file holds a version that is the person's.
pub(super) fn settle_kept_versions<S: Stores>(
    stores: &S,
    entry: &DirtyPublication,
) -> Result<(), RepairError> {
    let mut edit = None;
    for kept in stores
        .kept_beside(&entry.target)
        .map_err(RepairError::Stores)?
    {
        let current = stores
            .digest_of(&entry.target)
            .map_err(RepairError::Stores)?;
        let settled = match stores.digest_of(&kept).map_err(RepairError::Stores)? {
            None => Ok(()),
            Some(digest) if current.is_none() && Some(digest) == entry.expected_previous => {
                stores.return_kept(&kept, &entry.target)
            }
            Some(digest)
                if digest == entry.intended
                    || Some(digest) == entry.expected_previous
                    || Some(digest) == current =>
            {
                stores.remove_kept(&kept)
            }
            Some(_) => {
                edit = edit.or(Some(kept));
                Ok(())
            }
        };
        settled.map_err(RepairError::Stores)?;
    }
    let Some(kept) = edit else {
        return Ok(());
    };
    let versions = match stores
        .digest_of(&entry.target)
        .map_err(RepairError::Stores)?
    {
        Some(digest) if digest == entry.intended => KeptVersions::PublishedAtNote,
        Some(_) => KeptVersions::EditedNote,
        None => KeptVersions::NoteMissing,
    };
    Err(RepairError::Stores(StoreError::EditKept(KeptEdit {
        note: entry.target.clone(),
        kept,
        versions,
        cause: None,
    })))
}
