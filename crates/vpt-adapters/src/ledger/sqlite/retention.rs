use super::{SqliteLedger, map};
use rusqlite::{Row, Transaction, params, types::FromSql};
use std::path::PathBuf;
use vpt_application::ports::{
    ArtifactKind, LedgerError, NewIntent, RetentionIntent, RetentionJournal,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

fn corrupt(field: &str) -> LedgerError {
    LedgerError::Corrupt(format!("unreadable retention {field}"))
}

fn column<T: FromSql>(row: &Row<'_>, index: usize, field: &str) -> Result<T, LedgerError> {
    row.get(index).map_err(|_| corrupt(field))
}

fn intent_from_row(row: &Row<'_>) -> Result<RetentionIntent, LedgerError> {
    let id: i64 = column(row, 0, "identifier")?;
    if id <= 0 {
        return Err(corrupt("identifier"));
    }
    let kind: String = column(row, 1, "kind")?;
    let recording: String = column(row, 2, "recording")?;
    let path: String = column(row, 3, "path")?;
    let expected: String = column(row, 4, "digest")?;
    Ok(RetentionIntent {
        id,
        kind: ArtifactKind::parse(&kind).ok_or_else(|| corrupt("kind"))?,
        recording: RecordingId::parse(&recording).map_err(|_| corrupt("recording"))?,
        path: PathBuf::from(path),
        expected: Sha256Digest::from_hex(&expected).ok_or_else(|| corrupt("digest"))?,
        recorded_at: UtcInstant {
            secs: column(row, 5, "recorded time")?,
        },
    })
}

pub(super) fn insert(
    transaction: &Transaction<'_>,
    intent: &NewIntent,
) -> Result<i64, LedgerError> {
    let path = intent.path.to_str().ok_or_else(|| corrupt("path"))?;
    transaction
        .execute(
            "INSERT INTO retention_intents (artifact_kind, recording, path, expected, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                intent.kind.as_str(),
                intent.recording.as_str(),
                path,
                intent.expected.hex(),
                intent.recorded_at.secs,
            ],
        )
        .map_err(map)?;
    Ok(transaction.last_insert_rowid())
}

pub(super) fn complete(
    transaction: &Transaction<'_>,
    id: i64,
    at: UtcInstant,
) -> Result<(), LedgerError> {
    transaction
        .execute(
            "UPDATE retention_intents SET completed_at = ?2 WHERE id = ?1 AND completed_at IS NULL",
            params![id, at.secs],
        )
        .map(|_| ())
        .map_err(map)
}

impl RetentionJournal for SqliteLedger {
    fn record_intent(&self, intent: &NewIntent) -> Result<i64, LedgerError> {
        self.transaction(|transaction| insert(transaction, intent))
    }

    fn pending_intents(&self) -> Result<Vec<RetentionIntent>, LedgerError> {
        self.read(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT id, artifact_kind, recording, path, expected, recorded_at FROM retention_intents WHERE completed_at IS NULL ORDER BY id",
                )
                .map_err(map)?;
            let mut rows = statement.query([]).map_err(map)?;
            let mut intents = Vec::new();
            while let Some(row) = rows.next().map_err(map)? {
                intents.push(intent_from_row(row)?);
            }
            Ok(intents)
        })
    }

    fn complete_intent(&self, id: i64, at: UtcInstant) -> Result<(), LedgerError> {
        self.transaction(|transaction| complete(transaction, id, at))
    }
}
