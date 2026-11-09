use super::*;

pub(super) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    if let Some(change) = &delta.brief_published {
        let artifact =
            read::one(connection, "occasion", change.occasion.as_str())?.ok_or_else(conflict)?;
        let pending: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM dirty_publications WHERE target=?1 AND intended=?2)",
                params![
                    artifact.target.to_str().ok_or_else(conflict)?,
                    artifact.version.content.hex()
                ],
                |row| row.get(0),
            )
            .map_err(super::super::super::map)?;
        validation::published::check(delta, change, &artifact, pending, facts)?;
        connection
            .execute(
                "UPDATE owned_briefs SET published_at=?1 WHERE occasion=?2",
                params![change.at.secs, change.occasion.as_str()],
            )
            .map_err(super::super::super::map)?;
        connection
            .execute(
                "UPDATE occasions SET briefed_at=?1 WHERE id=?2",
                params![change.at.secs, change.occasion.as_str()],
            )
            .map_err(super::super::super::map)?;
    }
    Ok(())
}
