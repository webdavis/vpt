use super::super::map;
use crate::ledger::flags::{corrupt, prepare};
use rusqlite::{Connection, params};
use std::collections::HashSet;
use vpt_application::ports::{AcceptedTranscript, LedgerError};
use vpt_domain::{
    identity::RecordingId,
    review::{FlagContent, ReviewFlag},
};

pub(in crate::ledger::sqlite) fn replace(
    connection: &Connection,
    accepted: &AcceptedTranscript,
) -> Result<(), LedgerError> {
    let flags = prepare(accepted)?;
    let previous = super::all(connection, &accepted.recording)?;
    crate::ledger::flags::budget::check(
        flags.iter().chain(
            previous
                .iter()
                .filter(|flag| matches!(flag.content, FlagContent::Verification(_))),
        ),
    )?;
    let identifiers: HashSet<_> = flags.iter().map(|flag| flag.identifier).collect();
    for old in super::read(connection, &accepted.recording)? {
        if !identifiers.contains(&old.identifier) {
            connection
                .execute(
                    "DELETE FROM flags WHERE identifier = ?1 AND recording = ?2",
                    params![old.identifier.hex(), accepted.recording.as_str()],
                )
                .map_err(map)?;
        }
    }
    for flag in &flags {
        insert(connection, &accepted.recording, flag)?
    }
    crate::ledger::flags::budget::check(super::all(connection, &accepted.recording)?.iter())?;
    Ok(())
}

fn insert(connection: &Connection, id: &RecordingId, flag: &ReviewFlag) -> Result<(), LedgerError> {
    let (shape, class, ranges, record, alternative, confidence) = match &flag.content {
        FlagContent::Verification(_) => return Err(corrupt()),
        FlagContent::Lexical(flag) => (
            "lexical",
            flag.class.as_str(),
            Some(serde_json::to_string(&flag.ranges).map_err(|_| corrupt())?),
            Some(flag.record_text.as_str()),
            flag.alternative_text.as_deref(),
            flag.confidence,
        ),
        FlagContent::Diagnostic(diagnostic) => {
            ("diagnostic", diagnostic.as_str(), None, None, None, None)
        }
    };
    let changed = connection.execute(
        "INSERT INTO flags (identifier, recording, shape, class, ranges, record_text, alternative_text, confidence, state) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'open') \
         ON CONFLICT(identifier) DO UPDATE SET confidence=excluded.confidence \
         WHERE flags.recording=excluded.recording AND flags.shape=excluded.shape",
        params![flag.identifier.hex(), id.as_str(), shape, class, ranges, record, alternative, confidence]
    ).map_err(map)?;
    if changed != 1 {
        return Err(corrupt());
    }
    Ok(())
}
