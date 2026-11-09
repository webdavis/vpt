use super::*;
use crate::ledger::brief::{self as validation, conflict};
use rusqlite::{Connection, params};
use vpt_application::ports::WorkDelta;
use vpt_domain::{run::HistoryFact, time::UtcInstant};
mod publish;

pub(in crate::ledger::sqlite) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    validation::check_facts(delta, facts)?;
    validation::reservations::work(delta, |path| super::owns(connection, path))?;
    for path in &delta.publication_clears {
        if let Some(artifact) = read::one(
            connection,
            "target",
            path.to_str().ok_or_else(crate::ledger::brief::invalid)?,
        )? && delta
            .brief_published
            .as_ref()
            .is_none_or(|change| change.occasion != artifact.snapshot.id)
        {
            return Err(conflict());
        }
    }
    if let Some(change) = &delta.brief {
        let artifact = &change.artifact;
        let current = super::super::occasions::read::one(
            connection,
            "id = ?1",
            artifact.snapshot.id.as_str(),
        )?
        .ok_or_else(conflict)?;
        let old = read::one(connection, "occasion", artifact.snapshot.id.as_str())?;
        let encoded = validation::change::check(delta, change, &current, old.as_ref(), facts)?;
        let target = artifact.target.to_str().ok_or_else(conflict)?;
        let reserved:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM dirty_publications WHERE target=?1) OR EXISTS(SELECT 1 FROM retention_intents WHERE path=?1 AND completed_at IS NULL) OR EXISTS(SELECT 1 FROM owned_notes WHERE target=?1) OR EXISTS(SELECT 1 FROM engine_outputs WHERE target=?1) OR EXISTS(SELECT 1 FROM owned_briefs WHERE target=?1 AND occasion!=?2)",params![target,artifact.snapshot.id.as_str()],|row|row.get(0)).map_err(super::super::map)?;
        if reserved {
            return Err(conflict());
        }
        for source in &artifact.snapshot.pack.sources {
            let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM owned_notes WHERE recording=?1 AND stage='transcript' AND target=?2)",params![source.recording.as_str(),source.note.to_str().ok_or_else(conflict)?],|row|row.get(0)).map_err(super::super::map)?;
            if !exists {
                return Err(conflict());
            }
        }
        connection.execute("INSERT INTO owned_briefs (occasion,target,digest,snapshot_digest,body,managed_tags,first_written,write_offset,published_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,NULL) ON CONFLICT(occasion) DO UPDATE SET digest=excluded.digest,snapshot_digest=excluded.snapshot_digest,body=excluded.body,managed_tags=excluded.managed_tags,published_at=NULL",params![artifact.snapshot.id.as_str(),target,artifact.version.content.hex(),artifact.version.snapshot.hex(),artifact.text,crate::ledger::note::encode_tags(&artifact.managed_tags)?,artifact.first_written.secs,artifact.write_offset.secs]).map_err(super::super::map)?;
        connection
            .execute(
                "UPDATE occasions SET brief_path=?1,pack=?2 WHERE id=?3",
                params![target, encoded, artifact.snapshot.id.as_str()],
            )
            .map_err(super::super::map)?;
    }
    publish::apply(connection, delta, facts)
}
