use super::super::{map, migrations};
use crate::ledger::vocabulary::{invalid, validate};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use vpt_application::ports::{
    DirtyPublication, LedgerError, VocabularyKind, VocabularyPublication,
};
use vpt_domain::{digest::Sha256Digest, time::UtcInstant};

pub(super) fn target(
    connection: &Connection,
    target: &Path,
) -> Result<Option<VocabularyPublication>, LedgerError> {
    let version = migrations::current(connection)?;
    if version < 5 {
        return Ok(None);
    }
    let kind = if version < 7 {
        "'terms'"
    } else {
        "CASE WHEN typeof(n.kind) = 'text' AND length(CAST(n.kind AS BLOB)) <= 5 THEN n.kind END"
    };
    let sql = format!("SELECT
        CASE WHEN length(CAST(n.target AS BLOB)) <= 65536 THEN n.target END,
        CASE WHEN p.expected_previous IS NULL OR length(CAST(p.expected_previous AS BLOB)) = 64 THEN p.expected_previous ELSE X'00' END,
        CASE WHEN length(CAST(p.intended AS BLOB)) = 64 THEN p.intended END,
        p.recorded_at,
        CASE WHEN typeof(n.body) = 'blob' AND length(n.body) <= 16777216 THEN n.body END,
        {kind}
        FROM known_term_publications n LEFT JOIN dirty_publications p ON p.target = n.target WHERE n.target = ?1");
    let row = connection
        .query_row(&sql, [target.to_str().ok_or_else(invalid)?], |row| {
            for (column, limit) in [(0, 65_536), (2, 64), (4, 16_777_216)] {
                if row.get_ref(column)?.as_bytes()?.len() > limit {
                    return Err(rusqlite::Error::InvalidQuery);
                }
            }
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Vec<u8>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .optional()
        .map_err(|error| match error {
            rusqlite::Error::InvalidQuery
            | rusqlite::Error::InvalidColumnType(..)
            | rusqlite::Error::FromSqlConversionFailure(..)
            | rusqlite::Error::IntegralValueOutOfRange(..)
            | rusqlite::Error::Utf8Error(..) => invalid(),
            other => map(other),
        })?;
    row.map(|(target, previous, intended, at, bytes, kind)| {
        let digest = |text: &str| Sha256Digest::from_hex(text).ok_or_else(invalid);
        let value = VocabularyPublication {
            kind: match kind.as_str() {
                "terms" => VocabularyKind::Terms,
                "tags" => VocabularyKind::Tags,
                _ => return Err(invalid()),
            },
            entry: DirtyPublication {
                target: target.into(),
                expected_previous: previous.as_deref().map(digest).transpose()?,
                intended: digest(&intended)?,
                recorded_at: UtcInstant { secs: at },
            },
            bytes,
        };
        validate(&value)?;
        Ok(value)
    })
    .transpose()
}
