mod connection;
mod error;
mod migrations;

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
}

impl SqliteLedger {
    pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

    pub fn open(state: &RootDir) -> Result<Self, OpenError> {
        Self::open_with_timeout(state, Self::BUSY_TIMEOUT)
    }

    pub fn open_with_timeout(state: &RootDir, busy: Duration) -> Result<Self, OpenError> {
        connection::present(state)?;
        let ledger = Self {
            state: state.try_clone()?,
            path: state.path().join(DATABASE),
            busy,
            policy: AccessPolicy::WritableLedger,
        };
        ledger.read(|connection| {
            migrations::migrate(connection, || {
                ledger.state.revalidate().map_err(error::contained)
            })
        })?;
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

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Task 12 ledger repositories use transactions")
    )]
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
        let mut owner = connection::open(&self.state, self.busy, self.policy)?;
        let result = owner.with_connection(operation);
        error::finish(result, owner.try_close().map_err(Into::into))
    }
}

#[cfg(test)]
pub(super) mod tests;
