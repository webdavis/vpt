use super::*;
use crate::ledger::release::{self as release, conflict, validate};
use rusqlite::{OptionalExtension, params};
pub(super) fn apply(
    c: &Transaction<'_>,
    run: RunId,
    draft: &NewRelease,
) -> Result<PreparedRelease, LedgerError> {
    let last: Option<i64> = c
        .query_row(
            "SELECT seq FROM sqlite_sequence WHERE name='releases'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(map)?;
    let sequence = last
        .unwrap_or(0)
        .checked_add(1)
        .filter(|n| *n > 0)
        .ok_or_else(conflict)?;
    let recording = super::super::super::recordings::one_record(c, "id", draft.recording.as_str())?
        .ok_or_else(conflict)?;
    let row = validate::prepare(draft, sequence, &recording)?;
    let encoded = release::codec::encode(&row.provenance)?;
    for path in [&row.destination, &row.report_path] {
        let reserved: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM releases WHERE destination=?1 OR report_path=?1) OR EXISTS(SELECT 1 FROM dirty_publications WHERE target=?1) OR EXISTS(SELECT 1 FROM owned_notes WHERE target=?1) OR EXISTS(SELECT 1 FROM owned_briefs WHERE target=?1) OR EXISTS(SELECT 1 FROM engine_outputs WHERE target=?1) OR EXISTS(SELECT 1 FROM known_term_publications WHERE target=?1) OR EXISTS(SELECT 1 FROM retention_intents WHERE path=?1 AND completed_at IS NULL)", [validate::path(path)?], |r| r.get(0)).map_err(map)?;
        if reserved {
            return Err(conflict());
        }
    }
    c.execute("INSERT INTO releases (sequence,recording,stage,destination,content_digest,source_digest,report_path,snapshot,released_at,report_digest) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![row.sequence,row.recording.as_str(),row.stage.as_str(),validate::path(&row.destination)?,row.version.content.hex(),row.version.source.hex(),validate::path(&row.report_path)?,encoded,row.created_at.secs,row.version.report.hex()]).map_err(map)?;
    let [a, b] = release::intents(&row);
    let a = storage::record(c, run, row.created_at, a)?;
    let b = storage::record(c, run, row.created_at, b)?;
    c.execute("INSERT INTO release_payloads (sequence,body,report,copy_intent,report_intent) VALUES (?1,?2,?3,?4,?5)",params![row.sequence,draft.text,draft.report,storage::stored(a.id.sequence())?,storage::stored(b.id.sequence())?]).map_err(map)?;
    Ok(PreparedRelease {
        release: row,
        receipt: HistoryReceipt {
            run,
            records: vec![a, b],
            completed: vec![],
        },
    })
}
