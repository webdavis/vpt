use super::*;
use crate::ledger::brief::{codec, invalid, validate};
use rusqlite::{Connection, Row};
use vpt_application::ports::BriefVersion;
use vpt_domain::{
    digest::Sha256Digest,
    time::{UtcInstant, UtcOffset},
};

pub(super) fn one(
    connection: &Connection,
    predicate: &str,
    value: &str,
) -> Result<Option<BriefArtifact>, LedgerError> {
    if super::super::migrations::current(connection)? < 13 {
        return Ok(None);
    }
    let columns = [
        ("b.occasion", 30),
        ("b.target", 65_536),
        ("b.digest", 64),
        ("b.snapshot_digest", 64),
        ("o.pack", codec::MAX_BYTES),
        ("b.body", vpt_domain::note::MAX_RENDERED),
        ("b.managed_tags", crate::ledger::note::TAG_BYTES),
        ("o.brief_path", 65_536),
    ]
    .into_iter()
    .map(|(name, limit)| format!("CASE WHEN length(CAST({name} AS BLOB))<={limit} THEN {name} END"))
    .collect::<Vec<_>>()
    .join(",");
    let sql = format!(
        "SELECT {columns},b.first_written,b.write_offset,b.published_at,o.briefed_at FROM owned_briefs b LEFT JOIN occasions o ON o.id=b.occasion WHERE b.{predicate}=?1"
    );
    let mut statement = connection.prepare(&sql).map_err(super::super::map)?;
    let mut rows = statement.query([value]).map_err(super::super::map)?;
    rows.next()
        .map_err(super::super::map)?
        .map(decode)
        .transpose()
}
fn decode(row: &Row<'_>) -> Result<BriefArtifact, LedgerError> {
    let text = |i| {
        row.get_ref(i)
            .map_err(|_| invalid())?
            .as_str()
            .map_err(|_| invalid())
    };
    let artifact = BriefArtifact {
        snapshot: codec::decode(text(4)?)?,
        target: text(1)?.into(),
        version: BriefVersion {
            content: Sha256Digest::from_hex(text(2)?).ok_or_else(invalid)?,
            snapshot: Sha256Digest::from_hex(text(3)?).ok_or_else(invalid)?,
        },
        text: text(5)?.into(),
        managed_tags: crate::ledger::note::decode_tags(text(6)?)?,
        first_written: UtcInstant {
            secs: row.get(8).map_err(|_| invalid())?,
        },
        write_offset: UtcOffset {
            secs: row.get(9).map_err(|_| invalid())?,
        },
        published_at: row
            .get::<_, Option<i64>>(10)
            .map_err(|_| invalid())?
            .map(|secs| UtcInstant { secs }),
    };
    if artifact.snapshot.id.as_str() != text(0)?
        || artifact.target.as_os_str() != text(7)?
        || artifact
            .published_at
            .is_some_and(|at| row.get::<_, Option<i64>>(11).ok() != Some(Some(at.secs)))
    {
        return Err(invalid());
    }
    validate::validate(&artifact)?;
    Ok(artifact)
}
