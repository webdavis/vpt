use super::{SqliteLedger, connection, error, migrations};
use crate::database::{AccessPolicy, Database};
use std::fmt;
use vpt_application::ports::LedgerError;

pub(super) struct WalIndexLifetime {
    connection: Database,
}

impl fmt::Debug for WalIndexLifetime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WalIndexLifetime")
            .finish_non_exhaustive()
    }
}

impl WalIndexLifetime {
    pub(super) fn connection(&self) -> &Database {
        &self.connection
    }

    pub(super) fn initialize(ledger: &SqliteLedger) -> Result<Self, LedgerError> {
        let mut owner = connection::open(&ledger.state, ledger.busy, ledger.policy, None)?;
        let initialized = owner.with_connection(|connection| {
            migrations::migrate(connection, || {
                ledger.state.revalidate().map_err(error::contained)
            })
        });
        let retained = initialized.and_then(|()| Self::retain(ledger));
        error::finish(retained, owner.try_close().map_err(Into::into))
    }

    fn retain(ledger: &SqliteLedger) -> Result<Self, LedgerError> {
        // Keep SQLite's WAL lifetime locks while per-operation connections still close normally.
        // The completed read holds no transaction or WAL read mark between operations.
        let mut connection = connection::open(
            &ledger.state,
            ledger.busy,
            AccessPolicy::ObservationalLedger,
            None,
        )?;
        let initialized =
            connection.with_connection(|connection| migrations::current(connection).map(|_| ()));
        match initialized {
            Ok(()) => Ok(Self { connection }),
            Err(error) => error::finish(Err(error), connection.try_close().map_err(Into::into)),
        }
    }
}
