mod content;
mod verification;
use super::super::map;
use crate::ledger::flags::corrupt;
use rusqlite::{Connection, Row, types::ValueRef};
use vpt_application::ports::LedgerError;
use vpt_domain::{
    digest::Sha256Digest,
    identity::RecordingId,
    review::{FlagContent, Resolution, ReviewFlag},
    time::UtcInstant,
};

pub(in crate::ledger::sqlite) fn read(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Vec<ReviewFlag>, LedgerError> {
    read_flags(connection, id, false)
}

pub(in crate::ledger::sqlite) fn all(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Vec<ReviewFlag>, LedgerError> {
    let flags = read_flags(connection, id, true)?;
    for stage in [
        vpt_domain::note::NoteStage::Transcript,
        vpt_domain::note::NoteStage::Analysis,
    ] {
        if flags.iter().any(|flag| matches!(&flag.content, FlagContent::Verification(finding) if finding.artifact == stage)) {
            let owned: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM owned_notes WHERE recording=?1 AND stage=?2)", rusqlite::params![id.as_str(), stage.as_str()], |row| row.get(0)).map_err(map)?;
            if !owned { return Err(corrupt()); }
        }
    }
    Ok(flags)
}

fn read_flags(
    connection: &Connection,
    id: &RecordingId,
    include_verification: bool,
) -> Result<Vec<ReviewFlag>, LedgerError> {
    let mut statement = connection.prepare(
        "SELECT identifier, shape, class, ranges, record_text, alternative_text, confidence, state, resolution_text, resolved_at, artifact, claim_index, claim_digest \
         FROM flags WHERE recording = ?1 AND (?2 OR shape != 'verification') ORDER BY identifier LIMIT 65540"
    ).map_err(map)?;
    let mut rows = statement
        .query(rusqlite::params![id.as_str(), include_verification])
        .map_err(map)?;
    let mut result = Vec::new();
    let mut bytes = 0usize;
    while let Some(row) = rows.next().map_err(map)? {
        for index in 0..13 {
            let value = row.get_ref(index).map_err(|_| corrupt())?;
            if let ValueRef::Text(text) | ValueRef::Blob(text) = value {
                bytes = bytes.saturating_add(text.len());
            }
        }
        if bytes > 48 * 1024 * 1024 || result.len() == 65_539 {
            return Err(corrupt());
        }
        for index in 10..13 {
            if required(row, 1, 16)? == "verification" {
                break;
            }
            if row.get_ref(index).map_err(|_| corrupt())? != ValueRef::Null {
                return Err(corrupt());
            }
        }
        let identifier = Sha256Digest::from_hex(required(row, 0, 64)?).ok_or_else(corrupt)?;
        let content = if required(row, 1, 16)? == "verification" {
            verification::decode(row)?
        } else {
            content::decode(row)?
        };
        let expected = match &content {
            FlagContent::Lexical(flag) => crate::lexical_flag_id(id, flag),
            FlagContent::Diagnostic(diagnostic) => crate::diagnostic_flag_id(id, *diagnostic),
            FlagContent::Verification(finding) => crate::verification_flag_id(id, finding),
        };
        if expected != identifier {
            return Err(corrupt());
        }
        let resolution = resolution(row)?;
        if matches!(content, FlagContent::Diagnostic(_))
            && matches!(resolution, Resolution::Corrected { .. })
        {
            return Err(corrupt());
        }
        result.push(ReviewFlag {
            identifier,
            content,
            resolution,
        });
    }
    Ok(result)
}

fn resolution(row: &Row<'_>) -> Result<Resolution, LedgerError> {
    let state = required(row, 7, 16)?;
    let text = optional(row, 8, 262_144)?;
    let at = row
        .get::<_, Option<i64>>(9)
        .map_err(|_| corrupt())?
        .map(|secs| UtcInstant { secs });
    match (state, text, at) {
        ("open", None, None) => Ok(Resolution::Open),
        ("confirmed", None, Some(at)) => Ok(Resolution::Confirmed { at }),
        ("dismissed", None, Some(at)) => Ok(Resolution::Dismissed { at }),
        ("corrected", Some(text), Some(at)) if vpt_domain::review::valid_correction(text) => {
            Ok(Resolution::Corrected {
                text: text.into(),
                at,
            })
        }
        _ => Err(corrupt()),
    }
}

fn required<'a>(row: &'a Row<'_>, index: usize, maximum: usize) -> Result<&'a str, LedgerError> {
    optional(row, index, maximum)?.ok_or_else(corrupt)
}

fn optional<'a>(
    row: &'a Row<'_>,
    index: usize,
    maximum: usize,
) -> Result<Option<&'a str>, LedgerError> {
    match row.get_ref(index).map_err(|_| corrupt())? {
        ValueRef::Null => Ok(None),
        ValueRef::Text(bytes) if bytes.len() <= maximum => {
            std::str::from_utf8(bytes).map(Some).map_err(|_| corrupt())
        }
        _ => Err(corrupt()),
    }
}
