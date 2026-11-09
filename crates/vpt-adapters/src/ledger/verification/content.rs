use super::{conflict, invalid};
use crate::note_document::Document;
use sha2::{Digest, Sha256};
use vpt_application::ports::{LedgerError, NoteArtifact, VerificationChange, WorkDelta};
use vpt_domain::digest::Sha256Digest;

pub(super) fn check(
    change: &VerificationChange,
    delta: &WorkDelta,
    previous: &NoteArtifact,
    flags: &[vpt_domain::review::ReviewFlag],
) -> Result<Sha256Digest, LedgerError> {
    let source = if let Some(observed) = &change.observed {
        if observed.text.len() > vpt_domain::note::MAX_RENDERED
            || hash(observed.text.as_bytes()) != observed.digest
        {
            return Err(invalid());
        }
        let document = Document::parse(&observed.text).map_err(|_| invalid())?;
        document
            .check_owner(&change.recording, change.artifact.as_str())
            .map_err(|_| invalid())?;
        if hash(document.content().map_err(|_| invalid())?.as_bytes()) != change.input_digest {
            return Err(conflict());
        }
        let candidate = delta
            .notes
            .iter()
            .find(|note| {
                note.artifact.recording == change.recording
                    && note.artifact.stage == change.artifact
            })
            .ok_or_else(conflict)?;
        if candidate.expected_saved != Some(previous.digest)
            || candidate.artifact.target != previous.target
            || candidate.relocated_from.is_some()
            || !delta.commit.publications.iter().any(|entry| {
                entry.target == previous.target
                    && entry.expected_previous == Some(observed.digest)
                    && entry.intended == candidate.artifact.digest
            })
        {
            return Err(conflict());
        }
        observed.text.as_str()
    } else {
        if previous.content_digest != change.input_digest {
            return Err(conflict());
        }
        previous.text.as_str()
    };
    for candidate in &delta.notes {
        let next = &candidate.artifact;
        if next.recording == change.recording
            && next.stage == change.artifact
            && unannotated(source, change.artifact, flags)?
                != unannotated(&next.text, change.artifact, flags)?
        {
            return Err(conflict());
        }
    }
    let document = Document::parse(source).map_err(|_| invalid())?;
    crate::markdown_claims::extract_owned(
        document.content().map_err(|_| invalid())?,
        change.artifact,
        flags,
    )
    .map(|parsed| parsed.digest())
    .map_err(|_| invalid())
}

fn unannotated(
    source: &str,
    stage: vpt_domain::note::NoteStage,
    flags: &[vpt_domain::review::ReviewFlag],
) -> Result<String, LedgerError> {
    let document = Document::parse(source).map_err(|_| invalid())?;
    let claims = crate::markdown_claims::extract_owned(
        document.content().map_err(|_| invalid())?,
        stage,
        flags,
    )
    .map_err(|_| invalid())?;
    claims
        .annotate(&vec![None; claims.claims().len()])
        .map_err(|_| invalid())
}

fn hash(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}
