use super::super::{map, migrations};
use crate::ledger::note::{invalid, validate};
use rusqlite::{Connection, OptionalExtension, Params, params};
use std::path::Path;
use vpt_application::ports::{LedgerError, NoteArtifact, NoteStage};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    time::{UtcInstant, UtcOffset},
};

pub(in crate::ledger::sqlite) fn recording(
    connection: &Connection,
    id: &RecordingId,
    stage: NoteStage,
) -> Result<Option<NoteArtifact>, LedgerError> {
    read(
        connection,
        "recording = ?1 AND stage = ?2",
        params![id.as_str(), stage.as_str()],
    )
}
pub(super) fn target(
    connection: &Connection,
    path: &Path,
) -> Result<Option<NoteArtifact>, LedgerError> {
    read(
        connection,
        "target = ?1",
        params![path.to_str().ok_or_else(invalid)?],
    )
}

fn read(
    connection: &Connection,
    predicate: &str,
    values: impl Params,
) -> Result<Option<NoteArtifact>, LedgerError> {
    if migrations::current(connection)? < 4 {
        return Ok(None);
    }
    let sql = format!(
        "SELECT
        CASE WHEN length(CAST(target AS BLOB)) <= 65536 THEN target END,
        CASE WHEN length(CAST(recording AS BLOB)) <= 30 THEN recording END,
        CASE WHEN stage IN ('transcript', 'analysis') THEN stage END,
        CASE WHEN length(CAST(digest AS BLOB)) = 64 THEN digest END,
        CASE WHEN length(CAST(content_digest AS BLOB)) = 64 THEN content_digest END,
        CASE WHEN length(CAST(body AS BLOB)) <= 67108864 THEN body END,
        CASE WHEN length(CAST(managed_tags AS BLOB)) <= 16777216 THEN managed_tags END,
        first_written, write_offset, updated_at FROM owned_notes WHERE {predicate}"
    );
    let row = connection
        .query_row(&sql, values, |row| {
            for (column, maximum) in [
                (0, 65_536),
                (1, 30),
                (2, 10),
                (3, 64),
                (4, 64),
                (5, 67_108_864),
                (6, 16_777_216),
            ] {
                if row.get_ref(column)?.as_bytes()?.len() > maximum {
                    return Err(rusqlite::Error::InvalidQuery);
                }
            }
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i32>(8)?,
                row.get::<_, i64>(9)?,
            ))
        })
        .optional()
        .map_err(|error| match error {
            rusqlite::Error::InvalidQuery
            | rusqlite::Error::InvalidColumnType(..)
            | rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::IntegralValueOutOfRange(..) => invalid(),
            other => map(other),
        })?;
    row.map(
        |(target, recording, stage, digest, content_digest, text, tags, first, offset, updated)| {
            let note = NoteArtifact {
                target: target.into(),
                recording: RecordingId::parse(&recording).map_err(|_| invalid())?,
                stage: NoteStage::parse(&stage).ok_or_else(invalid)?,
                digest: Sha256Digest::from_hex(&digest).ok_or_else(invalid)?,
                content_digest: Sha256Digest::from_hex(&content_digest).ok_or_else(invalid)?,
                text,
                managed_tags: crate::ledger::note::decode_tags(&tags)?,
                first_written: UtcInstant { secs: first },
                write_offset: UtcOffset { secs: offset },
                updated_at: UtcInstant { secs: updated },
            };
            validate(&note)?;
            Ok(note)
        },
    )
    .transpose()
}
