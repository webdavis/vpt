use super::{SqliteLedger, connection, error, map};
use crate::sqlite_open::Mode;
use vpt_application::ports::LedgerError;

impl Drop for SqliteLedger {
    fn drop(&mut self) {
        if let Some(retained) = self.wal_lifetime.take() {
            let unchanged = connection::verify(&self.state, Some(retained.identity()));
            drop(retained);
            // A reader holding the ledger at this moment leaves the change in the write-ahead
            // log, where every command still reads it and the next writer moves it.
            if unchanged.is_ok() {
                let _ = into_database_file(self);
            }
        }
    }
}

/// Leaving WAL mode moves every committed change into `vpt.db` and empties the write-ahead log,
/// so `vpt.db` alone holds the ledger between commands.
fn into_database_file(ledger: &SqliteLedger) -> Result<(), LedgerError> {
    connection::verify(&ledger.state, None)?;
    let owner = connection::open_path(&ledger.state, ledger.busy, Mode::Writable)?;
    let result = owner
        .execute_batch("PRAGMA journal_size_limit=0")
        .and_then(|()| owner.query_row("PRAGMA journal_mode=DELETE", [], |_| Ok(())))
        .map_err(map);
    error::finish(result, connection::close(owner))
}
