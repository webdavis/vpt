use super::SqliteLedger;
mod read;
mod write;
use std::path::Path;
use vpt_application::ports::{LedgerError, VocabularyLedger, VocabularyPublication};
pub(super) use write::apply;

impl VocabularyLedger for SqliteLedger {
    fn vocabulary_publication(
        &self,
        target: &Path,
    ) -> Result<Option<VocabularyPublication>, LedgerError> {
        self.snapshot(|connection| read::target(connection, target))
    }
}

#[cfg(test)]
pub(super) mod tests;
