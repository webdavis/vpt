use super::map;
use crate::ledger::retention_artifacts::{MAX_ARTIFACTS, invalid};
use rusqlite::Connection;
use vpt_application::ports::*;
use vpt_domain::time::UtcInstant;

pub(super) fn overlay(
    connection: &Connection,
    version: u32,
    artifacts: &mut [RetentionArtifact],
) -> Result<(), LedgerError> {
    if version < 15 {
        return Ok(());
    }
    let mut statement = connection
        .prepare("SELECT path, kind, owner, digest, trashed_at FROM artifact_expirations LIMIT ?1")
        .map_err(map)?;
    let mut rows = statement
        .query([i64::try_from(MAX_ARTIFACTS + 1).map_err(|_| invalid())?])
        .map_err(map)?;
    let mut count = 0;
    while let Some(marker) = rows.next().map_err(map)? {
        count += 1;
        if count > MAX_ARTIFACTS {
            return Err(invalid());
        }
        let text = |column| {
            marker
                .get_ref(column)
                .ok()
                .and_then(|value| value.as_str().ok())
                .ok_or_else(invalid)
        };
        let path = std::path::Path::new(text(0)?);
        let index = artifacts
            .binary_search_by(|row| row.path.as_path().cmp(path))
            .map_err(|_| invalid())?;
        let row = &mut artifacts[index];
        if row.kind.as_str() != text(1)?
            || row.owner.as_str() != text(2)?
            || row.digest.hex() != text(3)?
        {
            return Err(invalid());
        }
        row.trashed_at = Some(UtcInstant {
            secs: marker.get(4).map_err(|_| invalid())?,
        });
        if row.status != RetentionStatus::PublicationPending {
            row.status = RetentionStatus::Expired;
        }
    }
    Ok(())
}

pub(super) fn regenerated(connection: &Connection, delta: &WorkDelta) -> Result<(), LedgerError> {
    for path in crate::ledger::retention_artifacts::completion::regenerated(delta) {
        connection
            .execute(
                "DELETE FROM artifact_expirations WHERE path=?1",
                [path.to_str().ok_or_else(invalid)?],
            )
            .map_err(map)?;
    }
    Ok(())
}
