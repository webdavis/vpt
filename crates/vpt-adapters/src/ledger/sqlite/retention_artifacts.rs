use super::{SqliteLedger, map};
use crate::ledger::retention_artifacts::{Inventory, MAX_ARTIFACTS, invalid};
use vpt_application::ports::*;
use vpt_domain::digest::Sha256Digest;
mod query;

impl RetentionArtifacts for SqliteLedger {
    fn retention_artifacts(&self) -> Result<Vec<RetentionArtifact>, LedgerError> {
        self.snapshot(read)
    }
}

pub(super) fn read(
    connection: &rusqlite::Connection,
) -> Result<Vec<RetentionArtifact>, LedgerError> {
    let version = super::migrations::current(connection)?;
    if version == 0 {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare(&query::sql(version)).map_err(map)?;
    let limit = i64::try_from(MAX_ARTIFACTS + 1).map_err(|_| invalid())?;
    let mut rows = statement.query([limit]).map_err(map)?;
    let mut inventory = Inventory::default();
    while let Some(row) = rows.next().map_err(map)? {
        let text = |column| {
            row.get_ref(column)
                .ok()
                .and_then(|value| value.as_str().ok())
                .ok_or_else(invalid)
        };
        let kind = ArtifactKind::parse(text(0)?).ok_or_else(invalid)?;
        let status = match row.get::<_, i64>(4).map_err(|_| invalid())? {
            0 => RetentionStatus::Published,
            1 => RetentionStatus::PublicationPending,
            2 => RetentionStatus::Expired,
            _ => return Err(invalid()),
        };
        inventory.push(
            kind,
            text(1)?,
            std::path::Path::new(text(2)?),
            Sha256Digest::from_hex(text(3)?).ok_or_else(invalid)?,
            status,
            row.get::<_, Option<i64>>(5)
                .map_err(|_| invalid())?
                .map(|secs| vpt_domain::time::UtcInstant { secs }),
        )?;
    }
    let mut rows = inventory.finish()?;
    super::expirations::overlay(connection, version, &mut rows)?;
    Ok(rows)
}
