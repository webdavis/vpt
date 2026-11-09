use super::*;
use vpt_domain::history::EventId;
pub(in crate::ledger::sqlite) fn read(
    c: &Connection,
    sequence: i64,
) -> Result<Option<ReleasePayload>, LedgerError> {
    let Some(release) = record(c, sequence)? else {
        return Ok(None);
    };
    if release.finalized_at.is_some() {
        return Ok(None);
    }
    let mut stmt = c.prepare("SELECT CASE WHEN length(CAST(body AS BLOB))<=16777216 THEN body END,CASE WHEN length(CAST(report AS BLOB))<=16777216 THEN report END,copy_intent,report_intent FROM release_payloads WHERE sequence=?1").map_err(map)?;
    let mut rows = stmt.query([sequence]).map_err(map)?;
    let row = rows.next().map_err(map)?.ok_or_else(invalid)?;
    let text: String = row.get(0).map_err(|_| invalid())?;
    let report: String = row.get(1).map_err(|_| invalid())?;
    if validate::digest(text.as_bytes()) != release.version.content
        || validate::digest(report.as_bytes()) != release.version.report
    {
        return Err(invalid());
    }
    let namespace = super::super::super::history::storage::namespace(c)?;
    let ids = [
        row.get::<_, i64>(2).map_err(|_| invalid())?,
        row.get::<_, i64>(3).map_err(|_| invalid())?,
    ];
    if ids.iter().any(|n| *n <= 0) || ids[0] == ids[1] {
        return Err(invalid());
    }
    Ok(Some(ReleasePayload {
        release,
        text,
        report,
        intents: ids.map(|id| EventId::new(namespace, id as u64)),
    }))
}
