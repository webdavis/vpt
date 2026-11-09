use crate::ports::{DirtyPublication, LedgerError, PublicationJournal, StoreError, Stores};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub completed: Vec<PathBuf>,
    pub republished: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RepairError {
    TargetModified(PathBuf),
    RenderedDigestMismatch(PathBuf),
    Sync { path: PathBuf, cause: StoreError },
    Render(PathBuf),
    Ledger(LedgerError),
    Stores(StoreError),
}

pub fn repair_publications<J: PublicationJournal, S: Stores>(
    journal: &J,
    stores: &S,
    render: impl Fn(&Path) -> Option<Vec<u8>>,
) -> Result<RepairReport, RepairError> {
    let mut report = RepairReport::default();
    for entry in journal
        .pending_publications()
        .map_err(RepairError::Ledger)?
    {
        let republished = repair_publication(stores, &entry, &render)?;
        journal
            .clear_publication(&entry.target)
            .map_err(RepairError::Ledger)?;
        if republished {
            report.republished.push(entry.target);
        } else {
            report.completed.push(entry.target);
        }
    }
    Ok(report)
}

pub fn repair_publication<S: Stores>(
    stores: &S,
    entry: &DirtyPublication,
    render: &impl Fn(&Path) -> Option<Vec<u8>>,
) -> Result<bool, RepairError> {
    let current = stores
        .digest_of(&entry.target)
        .map_err(RepairError::Stores)?;
    let republished = if current == Some(entry.intended) {
        false
    } else if current == entry.expected_previous {
        let bytes =
            render(&entry.target).ok_or_else(|| RepairError::Render(entry.target.clone()))?;
        if stores.digest_bytes(&bytes) != entry.intended {
            return Err(RepairError::RenderedDigestMismatch(entry.target.clone()));
        }
        stores
            .publish(&entry.target, entry.expected_previous, &bytes)
            .map_err(|error| match error {
                StoreError::Changed(path) => RepairError::TargetModified(path),
                other => RepairError::Stores(other),
            })?;
        true
    } else {
        return Err(RepairError::TargetModified(entry.target.clone()));
    };
    sync(stores, &entry.target)?;
    if stores
        .digest_of(&entry.target)
        .map_err(RepairError::Stores)?
        != Some(entry.intended)
    {
        return Err(RepairError::TargetModified(entry.target.clone()));
    }
    Ok(republished)
}

fn sync<S: Stores>(stores: &S, path: &Path) -> Result<(), RepairError> {
    stores
        .sync_directory_of(path)
        .map_err(|cause| RepairError::Sync {
            path: path.to_path_buf(),
            cause,
        })
}

#[cfg(test)]
mod tests;
