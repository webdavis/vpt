use super::conflict;
use std::path::Path;
use vpt_application::ports::{LedgerCommit, LedgerError, WorkDelta};

pub(crate) fn commit(
    batch: &LedgerCommit,
    mut owned: impl FnMut(&Path) -> Result<bool, LedgerError>,
) -> Result<(), LedgerError> {
    for entry in &batch.publications {
        if owned(&entry.target)? {
            return Err(conflict());
        }
    }
    Ok(())
}

pub(crate) fn work(
    delta: &WorkDelta,
    mut owned: impl FnMut(&Path) -> Result<bool, LedgerError>,
) -> Result<(), LedgerError> {
    for target in delta
        .notes
        .iter()
        .map(|row| &row.artifact.target)
        .chain(delta.engine_outputs.iter().map(|row| &row.target))
        .chain(delta.vocabulary.iter().map(|row| &row.entry.target))
    {
        if owned(target)? {
            return Err(conflict());
        }
    }
    for entry in &delta.commit.publications {
        if owned(&entry.target)?
            && delta
                .brief
                .as_ref()
                .is_none_or(|change| change.artifact.target != entry.target)
        {
            return Err(conflict());
        }
    }
    Ok(())
}
