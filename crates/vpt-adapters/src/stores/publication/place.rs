use super::{Checkpoint, replace::Hook, store_error};
use crate::contained::{ContainedError, Placement, RenameSupport, RootDir};
use std::path::Path;
use vpt_application::ports::StoreError;

/// Moves `from` to `to` only while `to` is absent. The volume must place exclusively.
pub(super) fn place_exclusive(
    root: &RootDir,
    support: RenameSupport,
    from: &Path,
    to: &Path,
    checkpoint: &mut Hook<'_>,
) -> Result<Placement, StoreError> {
    let failed = |path: &Path, error: ContainedError| store_error(&root.path().join(path), error);
    if support.exclusive {
        return match root.rename_exclusive(from, to) {
            Ok(true) => Ok(Placement::Placed),
            Ok(false) => Ok(Placement::Occupied),
            Err(error) => Err(failed(to, error)),
        };
    }
    let placement = root
        .link_exclusive(from, to)
        .map_err(|error| failed(to, error))?;
    if placement == Placement::Placed {
        checkpoint(Checkpoint::Linked)?;
        root.unlink_unchecked(from)
            .map_err(|error| failed(from, error))?;
    }
    Ok(placement)
}
