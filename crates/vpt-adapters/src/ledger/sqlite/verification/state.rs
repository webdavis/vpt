use crate::ledger::verification::{
    invalid,
    state::{self, VerifiedArtifact},
};
use rusqlite::{Connection, params};
use vpt_application::ports::{GroundingBasis, LedgerError, NoteStage, ReviewSnapshot, WorkDelta};
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    review::{FlagContent, Resolution},
    time::UtcInstant,
};

pub(in crate::ledger::sqlite) fn save(
    connection: &Connection,
    value: &VerifiedArtifact,
) -> Result<(), LedgerError> {
    connection.execute("INSERT INTO verified_artifacts (recording,artifact,input_digest,transcript_digest,lexical_digest,checked_at,claims_digest,grounding_window_secs) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
        ON CONFLICT(recording,artifact) DO UPDATE SET input_digest=excluded.input_digest,transcript_digest=excluded.transcript_digest,lexical_digest=excluded.lexical_digest,checked_at=excluded.checked_at,claims_digest=excluded.claims_digest,grounding_window_secs=excluded.grounding_window_secs",
        params![value.recording.as_str(),value.artifact.as_str(),value.input_digest.hex(),value.basis.transcript.hex(),value.basis.lexical_flags.hex(),value.at.secs,value.claims_digest.map(|digest|digest.hex()),value.grounding_window_secs]).map_err(super::super::map)?;
    Ok(())
}

pub(in crate::ledger::sqlite) fn read(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Vec<VerifiedArtifact>, LedgerError> {
    let version = super::super::migrations::current(connection)?;
    if version < 9 {
        return Ok(vec![]);
    }
    let sql = if version < 10 {
        "SELECT artifact,input_digest,transcript_digest,lexical_digest,checked_at,NULL,NULL FROM verified_artifacts WHERE recording=?1 LIMIT 3"
    } else if version < 11 {
        "SELECT artifact,input_digest,transcript_digest,lexical_digest,checked_at,claims_digest,NULL FROM verified_artifacts WHERE recording=?1 LIMIT 3"
    } else {
        "SELECT artifact,input_digest,transcript_digest,lexical_digest,checked_at,claims_digest,grounding_window_secs FROM verified_artifacts WHERE recording=?1 LIMIT 3"
    };
    let mut statement = connection.prepare(sql).map_err(super::super::map)?;
    let mut rows = statement.query([id.as_str()]).map_err(super::super::map)?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().map_err(super::super::map)? {
        if result.len() == 2 {
            return Err(invalid());
        }
        let text = |index, maximum| -> Result<&str, LedgerError> {
            match row.get_ref(index).map_err(|_| invalid())? {
                rusqlite::types::ValueRef::Text(bytes) if bytes.len() <= maximum => {
                    std::str::from_utf8(bytes).map_err(|_| invalid())
                }
                _ => Err(invalid()),
            }
        };
        let digest = |index| Sha256Digest::from_hex(text(index, 64)?).ok_or_else(invalid);
        let artifact = match text(0, 10)? {
            "transcript" => NoteStage::Transcript,
            "analysis" => NoteStage::Analysis,
            _ => return Err(invalid()),
        };
        result.push(VerifiedArtifact {
            recording: id.clone(),
            artifact,
            input_digest: digest(1)?,
            grounding_window_secs: match row.get_ref(6).map_err(|_| invalid())? {
                rusqlite::types::ValueRef::Null => None,
                rusqlite::types::ValueRef::Integer(value) => {
                    Some(value.try_into().map_err(|_| invalid())?)
                }
                _ => return Err(invalid()),
            },
            claims_digest: match row.get_ref(5).map_err(|_| invalid())? {
                rusqlite::types::ValueRef::Null => None,
                _ => Some(digest(5)?),
            },
            basis: GroundingBasis {
                transcript: digest(2)?,
                lexical_flags: digest(3)?,
            },
            at: UtcInstant {
                secs: row.get(4).map_err(|_| invalid())?,
            },
        });
    }
    Ok(result)
}

pub(in crate::ledger::sqlite) fn review(
    connection: &Connection,
    id: &RecordingId,
) -> Result<ReviewSnapshot, LedgerError> {
    let mut review = super::super::transcript::read_with_flags(connection, id)?;
    review.flags = super::super::flags::all(connection, id)?;
    state::partition(&mut review, &read(connection, id)?, |stage, flags| {
        super::super::notes::recording(connection, id, stage)?
            .as_ref()
            .map(|note| state::claims(note, flags))
            .transpose()
    })?;
    Ok(review)
}

pub(in crate::ledger::sqlite) fn refresh(
    connection: &Connection,
    delta: &WorkDelta,
) -> Result<(), LedgerError> {
    for id in state::affected(delta) {
        let review = review(connection, id)?;
        let open = review
            .flags
            .iter()
            .filter(|flag| {
                flag.resolution == Resolution::Open
                    && (matches!(&flag.content, FlagContent::Lexical(flag) if flag.class.surfaced())
                        || matches!(flag.content, FlagContent::Verification(_)))
            })
            .count();
        let diagnostics = review
            .flags
            .iter()
            .filter(|flag| {
                flag.resolution == Resolution::Open
                    && matches!(flag.content, FlagContent::Diagnostic(_))
            })
            .count();
        connection
            .execute(
                "UPDATE recordings SET open_flags=?2,diagnostics=?3 WHERE id=?1",
                params![id.as_str(), open as i64, diagnostics as i64],
            )
            .map_err(super::super::map)?;
        if let Some(proposal) = super::super::proposals::read(connection, id)? {
            let basis = review
                .accepted
                .as_ref()
                .map(|accepted| crate::ledger::proposal::basis::compute(accepted, &review.flags))
                .transpose()?;
            if Some(proposal.basis) != basis {
                connection.execute("UPDATE recordings SET stage_synthesis='pending' WHERE id=?1 AND stage_synthesis='succeeded'", [id.as_str()]).map_err(super::super::map)?;
            }
        }
    }
    Ok(())
}
