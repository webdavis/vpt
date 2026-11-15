use super::connection::{self, Identity};
use super::{SqliteLedger, error, migrations};
use rusqlite::Connection;
use std::fmt;
use vpt_application::ports::LedgerError;

/// Keeps one connection open for the ledger's life, so SQLite keeps the write-ahead log and its
/// index between the connections each operation opens and closes.
pub(super) struct WalIndexLifetime {
    _connection: Connection,
    identity: Identity,
}

impl fmt::Debug for WalIndexLifetime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WalIndexLifetime")
            .finish_non_exhaustive()
    }
}

impl WalIndexLifetime {
    pub(super) fn identity(&self) -> &Identity {
        &self.identity
    }

    pub(super) fn initialize(ledger: &SqliteLedger) -> Result<Self, LedgerError> {
        connection::verify(&ledger.state, None)?;
        connection::private(&ledger.state)?;
        let owner = connection::open(&ledger.state, ledger.busy, ledger.mode, None)?;
        let initialized = migrations::migrate(&owner, || {
            ledger.state.revalidate().map_err(error::contained)
        });
        let retained = initialized.and_then(|()| Self::retain(ledger));
        error::finish(retained, connection::close(owner))
    }

    fn retain(ledger: &SqliteLedger) -> Result<Self, LedgerError> {
        let retained = connection::open(&ledger.state, ledger.busy, ledger.mode, None)?;
        let initialized =
            migrations::current(&retained).and_then(|_| connection::verify(&ledger.state, None));
        match initialized {
            Ok(identity) => Ok(Self {
                _connection: retained,
                identity,
            }),
            Err(error) => error::finish(Err(error), connection::close(retained)),
        }
    }
}
