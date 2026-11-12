mod actions;
mod connection;
mod eligibility;
mod engine_outputs;
mod error;
mod expirations;
mod flags;
mod history;
mod journal;
mod migrations;
mod notes;
mod observation;
mod occasions;
mod proposals;
mod recordings;
mod relations;
mod retention;
mod retention_artifacts;
mod snapshot;
mod tags;
mod transcript;
mod verification;
mod vocabulary;
mod wal_lifetime;

use crate::database::AccessPolicy;
use crate::{ContainedError, RootDir};
use std::path::{Path, PathBuf};
use std::time::Duration;
use vpt_application::ports::LedgerError;

pub(crate) use error::map;
pub use migrations::VERSION as SCHEMA_VERSION;

const DATABASE: &str = "vpt.db";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    Contained(ContainedError),
    Ledger(LedgerError),
}

impl From<ContainedError> for OpenError {
    fn from(error: ContainedError) -> Self {
        Self::Contained(error)
    }
}

impl From<LedgerError> for OpenError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

#[derive(Debug)]
pub struct SqliteLedger {
    state: RootDir,
    path: PathBuf,
    busy: Duration,
    policy: AccessPolicy,
    wal_lifetime: Option<wal_lifetime::WalIndexLifetime>,
}

impl SqliteLedger {
    pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn open(state: &RootDir) -> Result<Self, OpenError> {
        Self::open_with_timeout(state, Self::BUSY_TIMEOUT)
    }

    pub fn open_with_timeout(state: &RootDir, busy: Duration) -> Result<Self, OpenError> {
        #[cfg(test)]
        let opening = tests::schema_diagnostics::start("ledger.open.start");
        connection::present(state)?;
        let mut ledger = Self {
            state: state.try_clone()?,
            path: state.path().join(DATABASE),
            busy,
            policy: AccessPolicy::WritableLedger,
            wal_lifetime: None,
        };
        ledger.wal_lifetime = Some(wal_lifetime::WalIndexLifetime::initialize(&ledger)?);
        #[cfg(test)]
        tests::schema_diagnostics::end(opening, "ledger.open.end");
        Ok(ledger)
    }

    pub fn open_read_only(state_dir: &Path) -> Result<Option<Self>, OpenError> {
        let state = match RootDir::open(state_dir) {
            Ok(state) => state,
            Err(ContainedError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            }) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if !connection::present(&state)? {
            return Ok(None);
        }
        let ledger = Self {
            path: state.path().join(DATABASE),
            state,
            busy: Self::BUSY_TIMEOUT,
            policy: AccessPolicy::ObservationalLedger,
            wal_lifetime: None,
        };
        let version = ledger.read(|connection| {
            let version = migrations::current(connection)?;
            if version != 0 {
                connection::companions(&ledger.state)?;
            }
            Ok(version)
        })?;
        Ok((version != 0).then_some(ledger))
    }

    pub fn database_path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> Result<u32, LedgerError> {
        self.read(migrations::current)
    }

    fn open_connection(&self) -> Result<crate::database::Database, LedgerError> {
        let retained = self
            .wal_lifetime
            .as_ref()
            .map(wal_lifetime::WalIndexLifetime::connection);
        connection::open(&self.state, self.busy, self.policy, retained)
    }

    pub(crate) fn transaction<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, LedgerError>,
    ) -> Result<T, LedgerError> {
        if self.policy != AccessPolicy::WritableLedger {
            return Err(LedgerError::Io("read-only ledger".into()));
        }
        self.read(|connection| {
            let transaction = rusqlite::Transaction::new_unchecked(
                connection,
                rusqlite::TransactionBehavior::Immediate,
            )
            .map_err(map)?;
            let value = operation(&transaction)?;
            self.state.revalidate().map_err(error::contained)?;
            transaction.commit().map_err(map)?;
            Ok(value)
        })
    }

    pub(crate) fn read<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Connection) -> Result<T, LedgerError>,
    ) -> Result<T, LedgerError> {
        #[cfg(test)]
        let reading = tests::schema_diagnostics::start("repo.read.start");
        #[cfg(test)]
        let opening = tests::schema_diagnostics::start("db.open.start");
        let mut owner = self.open_connection()?;
        #[cfg(test)]
        tests::schema_diagnostics::end(opening, "db.open.end");
        #[cfg(test)]
        let operating = tests::schema_diagnostics::start("db.operation.start");
        let result = owner.with_connection(operation);
        #[cfg(test)]
        tests::schema_diagnostics::end(operating, "db.operation.end");
        #[cfg(test)]
        let closing = tests::schema_diagnostics::start("db.close.start");
        let closed = owner.try_close().map_err(Into::into);
        #[cfg(test)]
        tests::schema_diagnostics::end(closing, "db.close.end");
        let result = error::finish(result, closed);
        #[cfg(test)]
        tests::schema_diagnostics::end(reading, "repo.read.end");
        result
    }
}

mod briefs;
mod releases;
#[cfg(test)]
pub(super) mod tests;
