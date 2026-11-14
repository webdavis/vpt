use super::{Checkpoint, FilesystemStores, displaced::Displaced, place, staged, store_error};
use crate::contained::{ContainedError, Placement, RenameSupport, RootDir};
use std::{fs::File, path::Path, path::PathBuf};
use vpt_application::ports::{StoreError, Stores};
use vpt_domain::digest::Sha256Digest;

pub(super) struct Replacement<'a> {
    pub stores: &'a FilesystemStores,
    pub root: &'a RootDir,
    pub support: RenameSupport,
    pub target: &'a Path,
    pub name: &'a str,
    pub note: &'a Path,
    pub staged: &'a Path,
    pub file: &'a File,
    pub bytes: &'a [u8],
    pub expected: Sha256Digest,
}

pub(super) type Hook<'h> = dyn FnMut(Checkpoint) -> Result<(), StoreError> + 'h;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AtNote {
    Unpublished,
    Published,
    Swapped,
}

impl Replacement<'_> {
    pub(super) fn run(&self, checkpoint: &mut Hook<'_>) -> Result<(), StoreError> {
        if self.support.swap && self.support.exclusive {
            self.swap_in(checkpoint)
        } else {
            self.set_aside_then_place(checkpoint)
        }
    }

    fn swap_in(&self, checkpoint: &mut Hook<'_>) -> Result<(), StoreError> {
        let kept = self.set_aside(self.staged)?.ok_or_else(|| self.changed())?;
        if let Err(error) = self.root.rename_swap(&kept, self.note) {
            let failed = match error {
                ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                } => self.changed(),
                error => store_error(self.target, error),
            };
            return Err(self.discard(&kept, failed));
        }
        checkpoint(Checkpoint::Displaced)?;
        let displaced = match Displaced::read(self.root, &kept) {
            Ok(displaced) => displaced,
            Err(error) => {
                let reason = self.unreadable(error);
                return Err(self.give_back(&kept, AtNote::Swapped, reason, checkpoint));
            }
        };
        if displaced.digest != self.expected {
            return Err(self.give_back(&kept, AtNote::Swapped, self.changed(), checkpoint));
        }
        checkpoint(Checkpoint::Hashed)?;
        self.finish(&kept, &displaced, AtNote::Swapped, checkpoint)
    }

    fn set_aside_then_place(&self, checkpoint: &mut Hook<'_>) -> Result<(), StoreError> {
        let kept = match self.set_aside(self.note) {
            Ok(Some(kept)) => kept,
            Ok(None) => return Err(self.discard(self.staged, self.changed())),
            Err(error) => {
                let reason = match self.stores.digest_of(self.target) {
                    Ok(Some(digest)) if digest == self.expected => error,
                    _ => self.changed(),
                };
                return Err(self.discard(self.staged, reason));
            }
        };
        checkpoint(Checkpoint::Displaced)?;
        let displaced = match Displaced::read(self.root, &kept) {
            Ok(displaced) if displaced.digest == self.expected => displaced,
            read => {
                let reason = read.map_or_else(|error| self.unreadable(error), |_| self.changed());
                let reason = self.discard(self.staged, reason);
                return Err(self.give_back(&kept, AtNote::Unpublished, reason, checkpoint));
            }
        };
        checkpoint(Checkpoint::Hashed)?;
        let placement =
            place::place_exclusive(self.root, self.support, self.staged, self.note, checkpoint);
        match placement {
            Ok(Placement::Placed) => self.finish(&kept, &displaced, AtNote::Published, checkpoint),
            Ok(Placement::Occupied) => {
                let reason = self.discard(self.staged, self.changed());
                self.finish(&kept, &displaced, AtNote::Unpublished, checkpoint)
                    .map_err(|error| match error {
                        StoreError::Changed(_) => reason,
                        error => error,
                    })
            }
            Err(failed) if self.note_holds_bytes() => self
                .finish(&kept, &displaced, AtNote::Published, checkpoint)
                .and(Err(failed)),
            Err(failed) => {
                let failed = self.discard(self.staged, failed);
                Err(self.give_back(&kept, AtNote::Unpublished, failed, checkpoint))
            }
        }
    }

    fn finish(
        &self,
        kept: &Path,
        displaced: &Displaced,
        at_note: AtNote,
        checkpoint: &mut Hook<'_>,
    ) -> Result<(), StoreError> {
        if !displaced.unchanged(self.root, kept) {
            return Err(self.give_back(kept, at_note, self.changed(), checkpoint));
        }
        self.root
            .unlink_unchecked(kept)
            .map_err(|error| store_error(kept, error))?;
        match at_note {
            AtNote::Unpublished => Err(self.changed()),
            AtNote::Published | AtNote::Swapped => Ok(()),
        }
    }

    pub(super) fn holds_staged(&self, path: &Path) -> bool {
        staged::validate(self.root, path, self.file, self.bytes).is_ok()
    }

    fn note_holds_bytes(&self) -> bool {
        let published = self.stores.digest_bytes(self.bytes);
        matches!(self.stores.digest_of(self.target), Ok(Some(digest)) if digest == published)
    }

    fn set_aside(&self, from: &Path) -> Result<Option<PathBuf>, StoreError> {
        self.stores
            .set_aside(self.root, self.support, from, self.name)
    }

    pub(super) fn changed(&self) -> StoreError {
        StoreError::Changed(self.target.to_path_buf())
    }
}
