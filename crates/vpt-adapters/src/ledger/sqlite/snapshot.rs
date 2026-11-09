use super::{SqliteLedger, error, map};
use rusqlite::{Connection, Transaction, TransactionBehavior};
use vpt_application::ports::LedgerError;

impl SqliteLedger {
    pub(super) fn snapshot<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, LedgerError>,
    ) -> Result<T, LedgerError> {
        self.read(|connection| {
            let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Deferred)
                .map_err(map)?;
            let result = operation(&transaction)?;
            self.state.revalidate().map_err(error::contained)?;
            transaction.commit().map_err(map)?;
            Ok(result)
        })
    }
}
