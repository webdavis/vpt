use super::super::{map, migrations};
use crate::ledger::note::{invalid, validate};
use rusqlite::{Connection, OptionalExtension, Params, Row, params};
use std::{ops::ControlFlow, path::Path};
use vpt_application::ports::{LedgerError, NoteArtifact, NoteStage};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    time::{UtcInstant, UtcOffset},
};

type Raw = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    i32,
    i64,
);

const COLUMNS: &str = "SELECT
        CASE WHEN length(CAST(n.target AS BLOB)) <= 65536 THEN n.target END,
        CASE WHEN length(CAST(n.recording AS BLOB)) <= 30 THEN n.recording END,
        CASE WHEN n.stage IN ('transcript', 'analysis') THEN n.stage END,
        CASE WHEN length(CAST(n.digest AS BLOB)) = 64 THEN n.digest END,
        CASE WHEN length(CAST(n.content_digest AS BLOB)) = 64 THEN n.content_digest END,
        CASE WHEN length(CAST(n.body AS BLOB)) <= 67108864 THEN n.body END,
        CASE WHEN length(CAST(n.managed_tags AS BLOB)) <= 16777216 THEN n.managed_tags END,
        n.first_written, n.write_offset, n.updated_at FROM owned_notes n";

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
pub(super) fn visit(
    connection: &Connection,
    visitor: &mut dyn FnMut(Result<NoteArtifact, LedgerError>) -> ControlFlow<()>,
) -> Result<(), LedgerError> {
    if migrations::current(connection)? < 4 {
        return Ok(());
    }
    let sql = format!(
        "{COLUMNS} JOIN recordings r ON r.id = n.recording
        ORDER BY r.captured_at DESC, r.id DESC, n.stage DESC"
    );
    let mut statement = connection.prepare(&sql).map_err(map)?;
    let mut rows = statement.query([]).map_err(map)?;
    while let Some(row) = rows.next().map_err(decode_error)? {
        let note = raw(row).map_err(decode_error).and_then(artifact);
        if visitor(note).is_break() {
            break;
        }
    }
    Ok(())
}

fn read(
    connection: &Connection,
    predicate: &str,
    values: impl Params,
) -> Result<Option<NoteArtifact>, LedgerError> {
    if migrations::current(connection)? < 4 {
        return Ok(None);
    }
    let sql = format!("{COLUMNS} WHERE {predicate}");
    connection
        .query_row(&sql, values, raw)
        .optional()
        .map_err(decode_error)?
        .map(artifact)
        .transpose()
}

fn raw(row: &Row<'_>) -> rusqlite::Result<Raw> {
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
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn decode_error(error: rusqlite::Error) -> LedgerError {
    match error {
        rusqlite::Error::InvalidQuery
        | rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::IntegralValueOutOfRange(..) => invalid(),
        other => map(other),
    }
}

fn artifact(
    (target, recording, stage, digest, content_digest, text, tags, first, offset, updated): Raw,
) -> Result<NoteArtifact, LedgerError> {
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
}
