use crate::ports::{LedgerError, PublicationJournal, StoreError, Stores};
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
        let current = stores
            .digest_of(&entry.target)
            .map_err(RepairError::Stores)?;
        if current == Some(entry.intended) {
            sync(stores, &entry.target)?;
            journal
                .clear_publication(&entry.target)
                .map_err(RepairError::Ledger)?;
            report.completed.push(entry.target);
        } else if current == entry.expected_previous {
            let bytes =
                render(&entry.target).ok_or_else(|| RepairError::Render(entry.target.clone()))?;
            if stores.digest_bytes(&bytes) != entry.intended {
                return Err(RepairError::RenderedDigestMismatch(entry.target));
            }
            stores
                .publish(&entry.target, &bytes)
                .map_err(RepairError::Stores)?;
            sync(stores, &entry.target)?;
            journal
                .clear_publication(&entry.target)
                .map_err(RepairError::Ledger)?;
            report.republished.push(entry.target);
        } else {
            return Err(RepairError::TargetModified(entry.target));
        }
    }
    Ok(report)
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
