use rusqlite::{Connection, Transaction, TransactionBehavior};
use vpt_application::ports::LedgerError;

pub const VERSION: u32 = 17;

pub(super) fn current(connection: &Connection) -> Result<u32, LedgerError> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(super::map)?;
    if version > VERSION {
        return Err(LedgerError::UnsupportedSchema(version));
    }
    Ok(version)
}

pub(super) fn migrate(
    connection: &Connection,
    before_commit: impl FnOnce() -> Result<(), LedgerError>,
) -> Result<(), LedgerError> {
    if current(connection)? == VERSION {
        return Ok(());
    }
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)
        .map_err(super::map)?;
    let version = current(&transaction)?;
    if version == VERSION {
        return Ok(());
    }
    if version == 0 {
        transaction.execute_batch(SCHEMA_V1).map_err(super::map)?;
    }
    if version < 2 {
        transaction.execute_batch(SCHEMA_V2).map_err(super::map)?;
    }
    if version < 3 {
        transaction.execute_batch(SCHEMA_V3).map_err(super::map)?;
    }
    if version < 4 {
        transaction.execute_batch(SCHEMA_V4).map_err(super::map)?;
    }
    if version < 5 {
        transaction.execute_batch(SCHEMA_V5).map_err(super::map)?;
    }
    if version < 6 {
        transaction.execute_batch(SCHEMA_V6).map_err(super::map)?;
    }
    if version < 7 {
        transaction.execute_batch(SCHEMA_V7).map_err(super::map)?;
    }
    if version < 8 {
        transaction.execute_batch(SCHEMA_V8).map_err(super::map)?;
    }
    if version < 9 {
        transaction.execute_batch(SCHEMA_V9).map_err(super::map)?;
    }
    if version < 10 {
        transaction.execute_batch(SCHEMA_V10).map_err(super::map)?;
    }
    if version < 11 {
        transaction.execute_batch(SCHEMA_V11).map_err(super::map)?;
    }
    if version < 12 {
        transaction.execute_batch(SCHEMA_V12).map_err(super::map)?;
    }
    if version < 13 {
        transaction.execute_batch(SCHEMA_V13).map_err(super::map)?;
    }
    if version < 14 {
        transaction.execute_batch(SCHEMA_V14).map_err(super::map)?;
    }
    if version < 15 {
        transaction.execute_batch(SCHEMA_V15).map_err(super::map)?;
    }
    if version < 16 {
        transaction.execute_batch(SCHEMA_V16).map_err(super::map)?;
    }
    transaction.execute_batch(SCHEMA_V17).map_err(super::map)?;
    transaction
        .pragma_update(None, "user_version", VERSION)
        .map_err(super::map)?;
    before_commit()?;
    transaction.commit().map_err(super::map)
}

pub(super) const SCHEMA_V1: &str = include_str!("migrations/v1.sql");
pub(super) const SCHEMA_V2: &str = include_str!("migrations/v2.sql");
const SCHEMA_V3: &str = include_str!("migrations/v3.sql");
const SCHEMA_V4: &str = include_str!("migrations/v4.sql");
const SCHEMA_V5: &str = include_str!("migrations/v5.sql");
const SCHEMA_V6: &str = include_str!("migrations/v6.sql");
const SCHEMA_V7: &str = include_str!("migrations/v7.sql");
const SCHEMA_V8: &str = include_str!("migrations/v8.sql");

const SCHEMA_V9: &str = include_str!("migrations/v9.sql");

const SCHEMA_V10: &str = include_str!("migrations/v10.sql");

const SCHEMA_V11: &str = include_str!("migrations/v11.sql");

const SCHEMA_V12: &str = include_str!("migrations/v12.sql");

const SCHEMA_V13: &str = include_str!("migrations/v13.sql");
const SCHEMA_V14: &str = include_str!("migrations/v14.sql");

const SCHEMA_V15: &str = include_str!("migrations/v15.sql");
const SCHEMA_V16: &str = include_str!("migrations/v16.sql");
const SCHEMA_V17: &str = include_str!("migrations/v17.sql");
