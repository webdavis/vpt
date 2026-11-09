use super::*;
use crate::ledger::release::{codec, invalid, validate};
use rusqlite::Connection;
use vpt_domain::{digest::Sha256Digest, time::UtcInstant};
mod payload;
pub(in crate::ledger::sqlite) use payload::read as payload;
pub(in crate::ledger::sqlite) fn record(
    c: &Connection,
    sequence: i64,
) -> Result<Option<ReleaseRecord>, LedgerError> {
    if super::super::migrations::current(c)? < 14 {
        return Ok(None);
    }
    let columns = [
        ("r.recording", 128),
        ("r.stage", 16),
        ("r.destination", 65_536),
        ("r.content_digest", 64),
        ("r.source_digest", 64),
        ("r.report_digest", 64),
        ("r.report_path", 65_536),
        ("r.snapshot", codec::MAX),
    ]
    .into_iter()
    .map(|(col, max)| format!("CASE WHEN length(CAST({col} AS BLOB))<={max} THEN {col} END"))
    .collect::<Vec<_>>()
    .join(",");
    let mut stmt = c.prepare(&format!("SELECT {columns},r.released_at,d.completed_at,EXISTS(SELECT 1 FROM release_payloads p WHERE p.sequence=r.sequence) FROM releases r LEFT JOIN release_completions d ON d.sequence=r.sequence WHERE r.sequence=?1")).map_err(map)?;
    let mut rows = stmt.query([sequence]).map_err(map)?;
    let Some(row) = rows.next().map_err(map)? else {
        return Ok(None);
    };
    let text = |i| {
        row.get_ref(i)
            .map_err(|_| invalid())?
            .as_str()
            .map_err(|_| invalid())
    };
    let sha = |i| Sha256Digest::from_hex(text(i)?).ok_or_else(invalid);
    let record = ReleaseRecord {
        sequence,
        recording: RecordingId::parse(text(0)?).map_err(|_| invalid())?,
        stage: NoteStage::parse(text(1)?).ok_or_else(invalid)?,
        destination: text(2)?.into(),
        version: ReleaseVersion {
            content: sha(3)?,
            source: sha(4)?,
            report: sha(5)?,
        },
        report_path: text(6)?.into(),
        provenance: codec::decode(text(7)?)?,
        created_at: UtcInstant {
            secs: row.get(8).map_err(|_| invalid())?,
        },
        finalized_at: row
            .get::<_, Option<i64>>(9)
            .map_err(|_| invalid())?
            .map(|secs| UtcInstant { secs }),
    };
    let pending: bool = row.get(10).map_err(|_| invalid())?;
    if pending == record.finalized_at.is_some() {
        return Err(invalid());
    }
    validate::record(&record)?;
    Ok(Some(record))
}
pub(super) fn sequences(
    c: &Connection,
    recording: Option<&RecordingId>,
) -> Result<Vec<i64>, LedgerError> {
    if super::super::migrations::current(c)? < 14 {
        return Ok(vec![]);
    }
    let sql = if recording.is_some() {
        "SELECT r.sequence FROM releases r JOIN release_completions c ON c.sequence=r.sequence WHERE r.recording=?1 ORDER BY r.sequence DESC LIMIT 65537"
    } else {
        "SELECT sequence FROM release_payloads WHERE ?1 IS NULL ORDER BY sequence LIMIT 65537"
    };
    let mut stmt = c.prepare(sql).map_err(map)?;
    let mut rows = stmt.query([recording.map(|r| r.as_str())]).map_err(map)?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().map_err(map)? {
        let id: i64 = row.get(0).map_err(|_| invalid())?;
        if id <= 0 || result.len() >= 65_536 {
            return Err(invalid());
        }
        result.push(id);
    }
    Ok(result)
}
