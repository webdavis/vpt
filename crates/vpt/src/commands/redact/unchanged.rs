use super::errors;
use std::path::Path;
use vpt_application::ports::ReleaseLedger;
use vpt_domain::{digest::Sha256Digest, identity::RecordingId, note::NoteStage};
use vpt_protocol::error::ErrorDocument;

pub(super) fn matches(
    ledger: &impl ReleaseLedger,
    id: &RecordingId,
    stage: NoteStage,
    destination: &Path,
    content: Sha256Digest,
) -> Result<bool, Box<ErrorDocument>> {
    for sequence in ledger.releases(id).map_err(errors::ledger)? {
        let release = ledger
            .release(sequence)
            .map_err(errors::ledger)?
            .ok_or_else(|| {
                errors::ledger(vpt_application::ports::LedgerError::Corrupt(
                    "release missing".into(),
                ))
            })?;
        if release.stage == stage
            && release.destination.parent() == Some(destination)
            && release.version.content == content
        {
            return Ok(true);
        }
    }
    Ok(false)
}
