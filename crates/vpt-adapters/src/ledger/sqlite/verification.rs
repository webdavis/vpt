pub(super) mod state;
use super::map;
use crate::ledger::verification::{conflict, invalid, prepare};
use rusqlite::{Connection, params};
use vpt_application::ports::{LedgerError, WorkDelta};
use vpt_domain::{
    review::{FlagContent, Resolution, ReviewFlag},
    run::HistoryFact,
    time::UtcInstant,
};

pub(super) fn apply(
    connection: &Connection,
    delta: &WorkDelta,
    facts: &[(UtcInstant, HistoryFact)],
) -> Result<(), LedgerError> {
    let Some(change) = &delta.verification else {
        return Ok(());
    };
    let note = super::notes::recording(connection, &change.recording, change.artifact)?
        .ok_or_else(conflict)?;
    let blocked: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM dirty_publications WHERE target=?1) OR EXISTS(SELECT 1 FROM retention_intents WHERE path=?1 AND completed_at IS NULL)",
        [note.target.to_str().ok_or_else(invalid)?], |row| row.get(0)).map_err(map)?;
    if blocked {
        return Err(conflict());
    }
    let review = super::transcript::read_with_flags(connection, &change.recording)?;
    let transcript = review.accepted.as_ref().ok_or_else(conflict)?;
    let flags = super::flags::all(connection, &change.recording)?;
    let (replacement, verified) = prepare(change, delta, facts, &note, transcript, &flags)?;
    replace(connection, &change.recording, change.artifact, &replacement)?;
    state::save(connection, &verified)
}

pub(super) fn replace(
    connection: &Connection,
    recording: &vpt_domain::identity::RecordingId,
    artifact: vpt_application::ports::NoteStage,
    replacement: &[ReviewFlag],
) -> Result<(), LedgerError> {
    connection
        .execute(
            "DELETE FROM flags WHERE recording=?1 AND shape='verification' AND artifact=?2",
            params![recording.as_str(), artifact.as_str()],
        )
        .map_err(map)?;
    for flag in replacement {
        insert(connection, recording, flag)?;
    }
    Ok(())
}

fn insert(
    connection: &Connection,
    id: &vpt_domain::identity::RecordingId,
    flag: &ReviewFlag,
) -> Result<(), LedgerError> {
    let FlagContent::Verification(finding) = &flag.content else {
        return Err(invalid());
    };
    let sources = if finding.sources.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(
                &finding
                    .sources
                    .iter()
                    .map(|source| [source.start_secs, source.end_secs])
                    .collect::<Vec<_>>(),
            )
            .map_err(|_| invalid())?,
        )
    };
    let token = match &finding.finding {
        vpt_domain::grounding::Finding::Unsupported(token) => Some(token.as_str()),
        _ => None,
    };
    let correction = match &flag.resolution {
        Resolution::Corrected { text, .. } => Some(text.as_str()),
        _ => None,
    };
    connection.execute("INSERT INTO flags (identifier,recording,shape,class,ranges,artifact,claim_index,claim_digest,record_text,alternative_text,state,resolution_text,resolved_at)
        VALUES (?1,?2,'verification',?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)", params![
            flag.identifier.hex(), id.as_str(), finding.finding.as_str(), sources, finding.artifact.as_str(), finding.claim_index,
            finding.claim_digest.hex(), finding.record_text, token,
            flag.resolution.kind().map_or("open", |kind| kind.as_str()), correction, flag.resolution.at().map(|at| at.secs)
        ]).map_err(map)?;
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
