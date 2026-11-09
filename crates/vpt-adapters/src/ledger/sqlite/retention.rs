use super::{SqliteLedger, map};
use rusqlite::{Row, Transaction, params, types::FromSql};
use std::path::PathBuf;
use vpt_application::ports::{
    ArtifactKind, ArtifactOwner, LedgerError, NewIntent, RetentionIntent, RetentionJournal,
};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::time::UtcInstant;

fn corrupt(field: &str) -> LedgerError {
    LedgerError::Corrupt(format!("unreadable retention {field}"))
}

fn column<T: FromSql>(row: &Row<'_>, index: usize, field: &str) -> Result<T, LedgerError> {
    row.get(index).map_err(|_| corrupt(field))
}

fn text<'a>(
    row: &'a Row<'_>,
    index: usize,
    field: &str,
    max: usize,
) -> Result<&'a str, LedgerError> {
    let value = row
        .get_ref(index)
        .map_err(|_| corrupt(field))?
        .as_str()
        .map_err(|_| corrupt(field))?;
    if value.len() > max {
        return Err(corrupt(field));
    }
    Ok(value)
}

pub(super) fn intent_from_row(row: &Row<'_>) -> Result<RetentionIntent, LedgerError> {
    let id: i64 = column(row, 0, "identifier")?;
    if id <= 0 {
        return Err(corrupt("identifier"));
    }
    let kind = ArtifactKind::parse(text(row, 1, "kind", 32)?).ok_or_else(|| corrupt("kind"))?;
    let owner = text(row, 2, "owner", 30)?;
    let path = text(row, 3, "path", 65_536)?;
    let expected = text(row, 4, "digest", 64)?;
    Ok(RetentionIntent {
        id,
        kind,
        owner: ArtifactOwner::parse(kind, owner).ok_or_else(|| corrupt("owner"))?,
        path: PathBuf::from(path),
        expected: Sha256Digest::from_hex(expected).ok_or_else(|| corrupt("digest"))?,
        recorded_at: UtcInstant {
            secs: column(row, 5, "recorded time")?,
        },
    })
}

pub(super) fn insert(
    transaction: &Transaction<'_>,
    intent: &NewIntent,
) -> Result<i64, LedgerError> {
    if !intent.owner.matches_kind(intent.kind) {
        return Err(LedgerError::Conflict(
            "retention owner kind mismatch".into(),
        ));
    }
    let path = intent.path.to_str().ok_or_else(|| corrupt("path"))?;
    transaction
        .execute(
            "INSERT INTO retention_intents (artifact_kind, recording, path, expected, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                intent.kind.as_str(),
                intent.owner.as_str(),
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
