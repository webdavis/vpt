use super::{SqliteLedger, map, migrations};
use vpt_application::ports::LedgerError;
use vpt_domain::ledger_identity::{LedgerIdentity, WriteMark};

const IDENTITY_SINCE: u32 = 20;

const STAMP: &str = "
UPDATE ledger_identity SET write_counter = write_counter + 1 WHERE id = 1;
INSERT INTO ledger_writes (write_counter, token)
SELECT write_counter, lower(hex(randomblob(16))) FROM ledger_identity WHERE id = 1;
DELETE FROM ledger_writes
WHERE write_counter <= (SELECT write_counter - 10000 FROM ledger_identity WHERE id = 1);
";

impl SqliteLedger {
    /// Returns nothing for a ledger older than the schema that gives it an identity.
    pub fn identity(&self) -> Result<Option<LedgerIdentity>, LedgerError> {
        self.read(|connection| {
            if migrations::current(connection)? < IDENTITY_SINCE {
                return Ok(None);
            }
            let (database_id, write_counter) = connection
                .query_row(
                    "SELECT database_id, write_counter FROM ledger_identity WHERE id = 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(map)?;
            let recent_tokens = connection
                .prepare("SELECT token FROM ledger_writes ORDER BY write_counter")
                .and_then(|mut tokens| tokens.query_map([], |row| row.get(0))?.collect())
                .map_err(map)?;
            Ok(Some(LedgerIdentity {
                database_id,
                write_counter,
                recent_tokens,
            }))
        })
    }

    /// Raises the write counter and records a new random token for this write.
    pub fn stamp(&self) -> Result<WriteMark, LedgerError> {
        self.transaction(|transaction| {
            transaction.execute_batch(STAMP).map_err(map)?;
            transaction
                .query_row(
                    "SELECT identity.database_id, identity.write_counter, writes.token \
                     FROM ledger_identity identity \
                     JOIN ledger_writes writes ON writes.write_counter = identity.write_counter",
                    [],
                    |row| {
                        Ok(WriteMark {
                            database_id: row.get(0)?,
                            write_counter: row.get(1)?,
                            write_token: row.get(2)?,
                        })
                    },
                )
                .map_err(map)
        })
    }
}
