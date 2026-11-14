use super::replace::{AtNote, Hook, Replacement};
use super::{Checkpoint, place, store_error};
use crate::contained::{ContainedError, Placement};
use std::path::Path;
use vpt_application::ports::{KeptEdit, KeptVersions, StoreError, Stores};

enum Left {
    Nothing,
    Kept(KeptVersions, Option<StoreError>),
    Failed(StoreError),
}

impl Replacement<'_> {
    /// Puts the displaced version back at the note where it can, and returns the error the
    /// publication ends with: `reason` when nothing stays kept.
    pub(super) fn give_back(
        &self,
        kept: &Path,
        at_note: AtNote,
        reason: StoreError,
        checkpoint: &mut Hook<'_>,
    ) -> StoreError {
        let left = match at_note {
            AtNote::Published => Left::Kept(KeptVersions::PublishedAtNote, None),
            AtNote::Swapped if self.holds_staged(self.note) => self.swap_back(kept, checkpoint),
            AtNote::Swapped | AtNote::Unpublished => self.put_back(kept, checkpoint),
        };
        match left {
            Left::Nothing => reason,
            Left::Kept(versions, cause) => StoreError::EditKept(KeptEdit {
                note: self.target.to_path_buf(),
                kept: kept.to_path_buf(),
                versions,
                cause: cause.map(Box::new),
            }),
            Left::Failed(error) => error,
        }
    }

    fn swap_back(&self, kept: &Path, checkpoint: &mut Hook<'_>) -> Left {
        let swapped = checkpoint(Checkpoint::SwapBack).and_then(|()| {
            self.root
                .rename_swap(kept, self.note)
                .map_err(|error| store_error(self.target, error))
        });
        if let Err(cause) = swapped {
            return Left::Kept(KeptVersions::PublishedAtNote, Some(cause));
        }
        if !self.holds_staged(kept) {
            return Left::Kept(KeptVersions::OlderSaveAtNote, None);
        }
        match self.root.unlink_file(kept) {
            Ok(()) => Left::Nothing,
            Err(error) => Left::Failed(store_error(kept, error)),
        }
    }

    fn put_back(&self, kept: &Path, checkpoint: &mut Hook<'_>) -> Left {
        match place::place_exclusive(self.root, self.support, kept, self.note, checkpoint) {
            Ok(Placement::Placed) => Left::Nothing,
            Ok(Placement::Occupied) => Left::Kept(KeptVersions::NewerSaveAtNote, None),
            Err(cause) => match self.stores.digest_of(self.target) {
                Ok(None) => Left::Kept(KeptVersions::NoteMissing, Some(cause)),
                _ => Left::Kept(KeptVersions::EditedNote, Some(cause)),
            },
        }
    }

    /// Removes vpt's own staged file. Its failure replaces `reason` only when `reason` merely
    /// says the note changed, so a refusal that names kept files always reaches the person.
    pub(super) fn discard(&self, path: &Path, reason: StoreError) -> StoreError {
        if !self.holds_staged(path) {
            return reason;
        }
        match (self.root.unlink_file(path), reason) {
            (Err(error), StoreError::Changed(_)) => {
                store_error(&self.root.path().join(path), error)
            }
            (_, reason) => reason,
        }
    }

    /// A displaced entry that is gone or is not a regular file was changed by someone else.
    pub(super) fn unreadable(&self, error: ContainedError) -> StoreError {
        match error {
            ContainedError::Io { kind, .. } if kind != std::io::ErrorKind::NotFound => {
                store_error(self.target, error)
            }
            _ => self.changed(),
        }
    }
}
