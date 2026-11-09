mod batch;
mod relocation;
mod target;
use crate::note_document::Document;
pub(super) use batch::check;
use sha2::{Digest, Sha256};
pub(super) use target::validate as validate_target;
use vpt_application::ports::{LedgerError, NoteArtifact};
use vpt_domain::digest::Sha256Digest;

pub(super) const TAG_BYTES: usize = 16_777_216;
pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid saved note".into())
}
pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("note publication conflict".into())
}

pub(super) fn validate(note: &NoteArtifact) -> Result<(), LedgerError> {
    if note.text.len() > vpt_domain::note::MAX_RENDERED
        || note.write_offset.secs.unsigned_abs() >= 86_400
        || note.managed_tags.len() > 256
        || Sha256Digest(Sha256::digest(note.text.as_bytes()).into()) != note.digest
    {
        return Err(invalid());
    }
    validate_target(&note.target)?;
    let parsed = Document::parse(&note.text).map_err(|_| invalid())?;
    parsed
        .check_owner(&note.recording, note.stage.as_str())
        .map_err(|_| invalid())?;
    if Sha256Digest(Sha256::digest(parsed.content().map_err(|_| invalid())?.as_bytes()).into())
        != note.content_digest
    {
        return Err(invalid());
    }
    let mut unique = std::collections::HashSet::new();
    for tag in &note.managed_tags {
        if tag.len() > 262_144
            || tag.chars().count() > 65_536
            || !unique.insert(tag)
            || !parsed.tags.contains(tag)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn encode_tags(tags: &Vec<String>) -> Result<String, LedgerError> {
    super::json::encode(tags, TAG_BYTES).map_err(|_| invalid())
}

pub(super) fn decode_tags(text: &str) -> Result<Vec<String>, LedgerError> {
    use vpt_protocol::{Limits, read_bounded};
    let value = read_bounded(
        text.as_bytes(),
        &Limits {
            bytes: TAG_BYTES,
            ..Limits::incoming()
        },
    )
    .map_err(|_| invalid())?;
    serde_json::from_value(value).map_err(|_| invalid())
}
