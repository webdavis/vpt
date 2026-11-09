mod codec;
mod predecessor;
pub(super) use predecessor::read as predecessor;
pub(super) mod write;

use super::{SqliteLedger, journal, map};
use codec::{RECORDING_COLUMNS, SEEN_COLUMNS, record_from_row, seen_from_row};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use vpt_application::ports::{
    LedgerCommit, LedgerError, RecordingLedger, RecordingRecord, SeenRow,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;
use write::{insert_recording, upsert_seen};

pub(super) fn one_record(
    connection: &Connection,
    key_column: &str,
    key: &str,
) -> Result<Option<RecordingRecord>, LedgerError> {
    connection
        .query_row(
            &format!("SELECT {RECORDING_COLUMNS} FROM recordings WHERE {key_column} = ?1"),
            [key],
            |row| Ok(record_from_row(row)),
        )
        .optional()
        .map_err(map)?
        .transpose()
}

pub(super) fn apply_commit(
    transaction: &rusqlite::Transaction<'_>,
    batch: &LedgerCommit,
) -> Result<(), LedgerError> {
    for row in &batch.seen {
        upsert_seen(transaction, row)?;
    }
    for entry in &batch.publications {
        journal::upsert_publication(transaction, entry)?;
    }
    for recording in &batch.recordings {
        insert_recording(transaction, recording)?;
    }
    Ok(())
}

impl RecordingLedger for SqliteLedger {
    fn seen(&self, path: &Path) -> Result<Option<SeenRow>, LedgerError> {
        self.read(|connection| {
            connection
                .query_row(
                    &format!("SELECT {SEEN_COLUMNS} FROM seen WHERE path = ?1"),
                    [path.to_string_lossy()],
                    |row| Ok(seen_from_row(row)),
                )
                .optional()
                .map_err(map)?
                .transpose()
        })
    }

    fn seen_all(&self) -> Result<Vec<SeenRow>, LedgerError> {
        self.read(|connection| {
            let mut statement = connection
                .prepare(&format!("SELECT {SEEN_COLUMNS} FROM seen ORDER BY path"))
                .map_err(map)?;
            let rows = statement
                .query_map([], |row| Ok(seen_from_row(row)))
                .map_err(map)?;
            rows.map(|row| row.map_err(map).and_then(|value| value))
                .collect()
        })
    }

    fn record_seen(&self, row: &SeenRow) -> Result<(), LedgerError> {
        self.transaction(|transaction| upsert_seen(transaction, row))
    }

    fn by_digest(&self, digest: &Sha256Digest) -> Result<Option<RecordingRecord>, LedgerError> {
        self.read(|connection| one_record(connection, "digest", &digest.hex()))
    }

    fn by_id(&self, id: &RecordingId) -> Result<Option<RecordingRecord>, LedgerError> {
        self.read(|connection| one_record(connection, "id", id.as_str()))
    }

    fn recordings(&self) -> Result<Vec<RecordingRecord>, LedgerError> {
        self.read(|connection| {
            let mut statement = connection
                .prepare(&format!(
                    "SELECT {RECORDING_COLUMNS} FROM recordings ORDER BY captured_at, id"
                ))
                .map_err(map)?;
            let rows = statement
                .query_map([], |row| Ok(record_from_row(row)))
                .map_err(map)?;
            rows.map(|row| row.map_err(map).and_then(|value| value))
                .collect()
        })
    }

    fn commit(&self, batch: &LedgerCommit) -> Result<(), LedgerError> {
        self.transaction(|transaction| {
            apply_commit(transaction, batch)
        })
    }

    fn set_source_path(&self, id: &RecordingId, path: &Path) -> Result<(), LedgerError> {
        self.transaction(|transaction| {
            transaction
                .execute(
                    "UPDATE recordings SET source_path = ?1 WHERE id = ?2",
                    params![path.to_string_lossy(), id.as_str()],
                )
                .map(|_| ())
                .map_err(map)
        })
    }

    fn set_audio_trashed(&self, id: &RecordingId, at: UtcInstant) -> Result<(), LedgerError> {
        self.transaction(|transaction| {
            transaction
                .execute(
                    "UPDATE recordings SET audio_trashed_at = ?1 WHERE id = ?2",
                    params![at.secs, id.as_str()],
                )
                .map(|_| ())
                .map_err(map)
        })
    }
}
