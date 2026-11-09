use super::RuntimeLedger;
use vpt_application::ports::{LedgerError, RetentionArtifact, RetentionArtifacts};
impl RetentionArtifacts for RuntimeLedger {
    fn retention_artifacts(&self) -> Result<Vec<RetentionArtifact>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.retention_artifacts(),
            Self::Empty(ledger) => ledger.retention_artifacts(),
        }
    }
}
