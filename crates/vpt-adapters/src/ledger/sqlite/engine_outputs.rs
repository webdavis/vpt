use super::{SqliteLedger, map};
use crate::ledger::engine_output::{self as validation, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use vpt_application::ports::{EngineArtifact, EngineOutputLedger, LedgerError, WorkDelta};
use vpt_domain::transcript::LanguageSource;
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, time::UtcInstant};

impl EngineOutputLedger for SqliteLedger {
    fn engine_output(&self, target: &Path) -> Result<Option<EngineArtifact>, LedgerError> {
        self.read(|connection| {
            if super::migrations::current(connection)? < 3 {
                return Ok(None);
            }
            let mut statement = connection
                .prepare(
                    "SELECT
                   CASE WHEN length(CAST(recording AS BLOB)) <= 30 THEN recording END,
                   CASE WHEN length(CAST(engine AS BLOB)) <= 64 THEN engine END,
                   CASE WHEN length(CAST(digest AS BLOB)) <= 64 THEN digest END,
                   duration, CASE WHEN length(body) <= 16777216 THEN body END, created_at,
                   CASE WHEN language_source IN ('reported', 'requested') THEN language_source END
                 FROM engine_outputs WHERE target = ?1",
                )
                .map_err(map)?;
            let row = statement
                .query_row([target.to_str().ok_or_else(invalid)?], |row| {
                    // Inspect borrowed SQLite values before allocating owned strings or bytes.
                    for (column, limit) in [(0, 30), (1, 64), (2, 64), (4, 16_777_216)] {
                        if row.get_ref(column)?.as_bytes()?.len() > limit {
                            return Err(rusqlite::Error::InvalidQuery);
                        }
                    }
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                })
                .optional()
                .map_err(|error| match error {
                    rusqlite::Error::InvalidQuery
                    | rusqlite::Error::InvalidColumnType(..)
                    | rusqlite::Error::FromSqlConversionFailure(..) => invalid(),
                    other => map(other),
                })?;
            row.map(|(recording, engine, digest, duration, bytes, at, source)| {
                let output = EngineArtifact {
                    language_source: match source.as_str() {
                        "reported" => LanguageSource::Reported,
                        "requested" => LanguageSource::Requested,
                        _ => return Err(invalid()),
                    },
                    recording: RecordingId::parse(&recording).map_err(|_| invalid())?,
                    engine,
                    target: target.into(),
                    digest: Sha256Digest::from_hex(&digest).ok_or_else(invalid)?,
                    duration,
                    bytes,
                    created_at: UtcInstant { secs: at },
                };
                validation::validate(&output)?;
                Ok(output)
            })
            .transpose()
        })
    }
}

pub(super) fn check(connection: &Connection, delta: &WorkDelta) -> Result<(), LedgerError> {
    validation::check_batch(delta, |target| {
        let path = target.to_str().ok_or_else(invalid)?;
        let digest: Option<String> = connection
            .query_row(
                "SELECT CASE WHEN length(CAST(digest AS BLOB)) <= 64 THEN digest END FROM engine_outputs WHERE target = ?1",
                [path],
                |row| row.get(0),
            )
            .optional()
            .map_err(map)?;
        let pending: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM dirty_publications WHERE target = ?1) OR EXISTS(SELECT 1 FROM retention_intents WHERE path = ?1 AND completed_at IS NULL)",
                [path],
                |row| row.get(0),
            )
            .map_err(map)?;
        Ok((
            digest
                .map(|value| Sha256Digest::from_hex(&value).ok_or_else(invalid))
                .transpose()?,
            pending,
        ))
    })
}

pub(super) fn apply(connection: &Connection, delta: &WorkDelta) -> Result<(), LedgerError> {
    for output in &delta.engine_outputs {
        connection.execute(
            "INSERT INTO engine_outputs (target, recording, engine, digest, duration, body, created_at, language_source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
             ON CONFLICT(target) DO UPDATE SET recording=excluded.recording, engine=excluded.engine, digest=excluded.digest, duration=excluded.duration, body=excluded.body, created_at=excluded.created_at, language_source=excluded.language_source",
            params![output.target.to_str().ok_or_else(invalid)?, output.recording.as_str(), output.engine, output.digest.hex(), output.duration, output.bytes, output.created_at.secs, match output.language_source {LanguageSource::Reported => "reported", LanguageSource::Requested => "requested"}],
        ).map_err(map)?;
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
