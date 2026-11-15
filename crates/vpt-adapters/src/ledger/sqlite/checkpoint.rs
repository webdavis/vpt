use super::wal_lifetime::WalIndexLifetime;
use super::{DATABASE, SqliteLedger, error, map};
use crate::database::{AccessPolicy, Database};
use std::path::Path;
use vpt_application::ports::LedgerError;

impl Drop for SqliteLedger {
    fn drop(&mut self) {
        if let Some(retained) = self.wal_lifetime.take() {
            let unchanged = unchanged(self, &retained);
            drop(retained);
            // A reader holding the ledger at this moment leaves the change in the write-ahead
            // log, where every command still reads it and the next writer moves it.
            if unchanged.is_ok() {
                let _ = into_database_file(self);
            }
        }
    }
}

fn unchanged(ledger: &SqliteLedger, retained: &WalIndexLifetime) -> Result<(), LedgerError> {
    let root = ledger.state.try_clone().map_err(error::contained)?;
    let mut pinned = Database::open_pinned(
        root,
        Path::new(DATABASE),
        AccessPolicy::WritableLedger,
        ledger.busy,
        retained.connection(),
    )?;
    Ok(pinned.try_close()?)
}

/// Leaving WAL mode moves every committed change into `vpt.db` and empties the write-ahead log,
/// so `vpt.db` alone holds the ledger between commands.
fn into_database_file(ledger: &SqliteLedger) -> Result<(), LedgerError> {
    let root = ledger.state.try_clone().map_err(error::contained)?;
    let mut owner = Database::open(
        root,
        Path::new(DATABASE),
        AccessPolicy::WritableLedger,
        ledger.busy,
    )?;
    let result = owner.with_connection(|connection| {
        connection
            .execute_batch("PRAGMA journal_size_limit=0")
            .and_then(|()| connection.query_row("PRAGMA journal_mode=DELETE", [], |_| Ok(())))
            .map_err(map)
    });
    error::finish(result, owner.try_close().map_err(Into::into))
}
