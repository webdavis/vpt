use super::{MemoryLedger, with};
use std::path::Path;
use vpt_application::ports::{DirtyPublication, LedgerError, PublicationJournal};
impl PublicationJournal for MemoryLedger {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
        with(self, |state| {
            let mut entries = state.publications.clone();
            entries.sort_by(|left, right| {
                (left.recorded_at, left.target.as_os_str())
                    .cmp(&(right.recorded_at, right.target.as_os_str()))
            });
            entries
        })
    }

    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
        with(self, |state| {
            state
                .vocabulary
                .retain(|entry| entry.entry.target != target);
            state.publications.retain(|entry| entry.target != target);
            Ok(())
        })?
    }
}
