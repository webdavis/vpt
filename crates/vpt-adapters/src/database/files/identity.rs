use super::{ContainedError, DatabaseError, DatabaseFiles, FileRole};
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

impl DatabaseFiles {
    pub(in crate::database) fn pin_to(&self, owner: &Self) -> Result<(), DatabaseError> {
        let opened = owner.opened.get();
        if opened.iter().any(Option::is_none) {
            return Err(super::super::invalid("uninitialized WAL identity"));
        }
        self.expected.set(opened);
        for role in [FileRole::Database, FileRole::Wal, FileRole::SharedMemory] {
            if self.metadata(role, false)?.is_none() {
                return Err(self.identity_changed(role));
            }
        }
        Ok(())
    }

    pub(super) fn requires_existing(&self, role: FileRole) -> bool {
        slot(role).is_some_and(|index| self.expected.get()[index].is_some())
    }

    pub(super) fn record_identity(
        &self,
        role: FileRole,
        metadata: &Metadata,
    ) -> Result<(), DatabaseError> {
        self.check_identity(role, metadata)?;
        let Some(index) = slot(role) else {
            return Ok(());
        };
        let identity = (metadata.dev(), metadata.ino());
        let mut opened = self.opened.get();
        opened[index].get_or_insert(identity);
        self.opened.set(opened);
        Ok(())
    }

    pub(super) fn check_identity(
        &self,
        role: FileRole,
        metadata: &Metadata,
    ) -> Result<(), DatabaseError> {
        let identity = (metadata.dev(), metadata.ino());
        if slot(role).is_some_and(|index| {
            self.expected.get()[index].is_some_and(|expected| expected != identity)
        }) {
            return Err(self.identity_changed(role));
        }
        Ok(())
    }

    pub(super) fn identity_changed(&self, role: FileRole) -> DatabaseError {
        ContainedError::Escape {
            root: self.root.path().into(),
            path: self.root.path().join(self.name(role)),
        }
        .into()
    }
}

fn slot(role: FileRole) -> Option<usize> {
    match role {
        FileRole::Database => Some(0),
        FileRole::Wal => Some(1),
        FileRole::SharedMemory => Some(2),
        FileRole::Journal => None,
    }
}
