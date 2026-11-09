use super::{SqliteLedger, map};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use vpt_application::ports::{DirtyPublication, LedgerError, PublicationJournal};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::UtcInstant;

fn column_error(error: rusqlite::Error) -> LedgerError {
    match error {
        rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::IntegralValueOutOfRange(..)
        | rusqlite::Error::Utf8Error(..) => {
            LedgerError::Corrupt("unreadable publication column".into())
        }
        other => map(other),
    }
}

pub(super) fn upsert_publication(
    connection: &Connection,
    entry: &DirtyPublication,
) -> Result<(), LedgerError> {
    if super::releases::owns(connection, &entry.target)? {
        return Err(crate::ledger::release::conflict());
    }
    if super::migrations::current(connection)? >= 5
        && connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM known_term_publications WHERE target=?1)",
                [entry.target.to_string_lossy()],
                |row| row.get::<_, bool>(0),
            )
            .map_err(map)?
    {
        return Err(crate::ledger::vocabulary::conflict());
    }
    connection
        .execute(
            "INSERT INTO dirty_publications (target, expected_previous, intended, recorded_at) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(target) DO UPDATE SET expected_previous = excluded.expected_previous, \
             intended = excluded.intended, recorded_at = excluded.recorded_at",
            params![
                entry.target.to_string_lossy(),
                entry.expected_previous.map(|digest| digest.hex()),
                entry.intended.hex(),
                entry.recorded_at.secs,
            ],
        )
        .map(|_| ())
        .map_err(map)
}

impl PublicationJournal for SqliteLedger {
    fn pending_publications(&self) -> Result<Vec<DirtyPublication>, LedgerError> {
        self.read(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT target, expected_previous, intended, recorded_at \
                     FROM dirty_publications ORDER BY recorded_at, target",
                )
                .map_err(map)?;
            let mut rows = statement.query([]).map_err(map)?;
            let mut publications = Vec::new();
            while let Some(row) = rows.next().map_err(map)? {
                let target: String = row.get(0).map_err(column_error)?;
                let previous: Option<String> = row.get(1).map_err(column_error)?;
                let intended: String = row.get(2).map_err(column_error)?;
                let recorded_at: i64 = row.get(3).map_err(column_error)?;
                let decode = |text: &str| {
                    Sha256Digest::from_hex(text)
                        .ok_or_else(|| LedgerError::Corrupt("unreadable publication digest".into()))
                };
                publications.push(DirtyPublication {
                    target: PathBuf::from(target),
                    expected_previous: previous.as_deref().map(decode).transpose()?,
                    intended: decode(&intended)?,
                    recorded_at: UtcInstant { secs: recorded_at },
                });
            }
            Ok(publications)
        })
    }

    fn clear_publication(&self, target: &Path) -> Result<(), LedgerError> {
        self.transaction(|transaction| {
            if super::releases::owns(transaction, target)? {
                return Err(crate::ledger::release::conflict());
            }
            let owned: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM owned_briefs WHERE target=?1)",
                    [target.to_string_lossy()],
                    |row| row.get(0),
                )
                .map_err(map)?;
            if owned {
                return Err(crate::ledger::brief::conflict());
            }

            transaction
                .execute(
                    "DELETE FROM dirty_publications WHERE target = ?1",
                    [target.to_string_lossy()],
                )
                .map(|_| ())
                .map_err(map)
        })
    }
}
