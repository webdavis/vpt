use super::*;
use sha2::{Digest, Sha256};
use vpt_application::ports::BriefArtifact;
use vpt_domain::{
    digest::Sha256Digest,
    time::{UtcInstant, UtcOffset},
};

pub(crate) fn validate(artifact: &BriefArtifact) -> Result<String, LedgerError> {
    let encoded = codec::encode(&artifact.snapshot)?;
    if artifact.text.len() > vpt_domain::note::MAX_RENDERED
        || digest(artifact.text.as_bytes()) != artifact.version.content
        || digest(encoded.as_bytes()) != artifact.version.snapshot
        || artifact.write_offset.secs.unsigned_abs() >= 86_400
        || !instant(artifact.first_written)
        || artifact.published_at.is_some_and(|at| !instant(at))
        || artifact.managed_tags.len() > 256
    {
        return Err(invalid());
    }
    vpt_domain::brief::document::source_path(&artifact.target).ok_or_else(invalid)?;
    crate::ledger::note::validate_target(&artifact.target)?;
    let parsed =
        crate::note_document::brief::Document::parse(&artifact.text).map_err(|_| invalid())?;
    parsed
        .check_owner(&artifact.snapshot.id)
        .map_err(|_| invalid())?;
    let mut unique = std::collections::BTreeSet::new();
    for tag in &artifact.managed_tags {
        if tag.len() > 4 * 65_536
            || tag.chars().count() > 65_536
            || !unique.insert(tag)
            || !parsed.tags.contains(tag)
        {
            return Err(invalid());
        }
    }
    Ok(encoded)
}
pub(crate) fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}
pub(crate) fn instant(at: UtcInstant) -> bool {
    (0..=9999).contains(&at.civil(UtcOffset { secs: 0 }).year)
}
