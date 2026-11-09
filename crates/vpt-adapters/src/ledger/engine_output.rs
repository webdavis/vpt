use sha2::{Digest, Sha256};
use std::path::{Component, Path};
use vpt_application::ports::{EngineArtifact, LedgerError, TranscriptionChange, WorkDelta};
use vpt_domain::digest::Sha256Digest;
use vpt_protocol::engine::{self, ExpectedEngine};

pub(super) fn invalid() -> LedgerError {
    LedgerError::Corrupt("invalid saved engine output".into())
}
pub(super) fn conflict() -> LedgerError {
    LedgerError::Conflict("engine output publication conflict".into())
}

pub(super) fn validate(output: &EngineArtifact) -> Result<engine::EngineDocument, LedgerError> {
    if output.engine.is_empty()
        || output.engine.len() > 64
        || !output
            .engine
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        || output.bytes.len() > 16_777_216
        || !output.target.is_absolute()
        || output
            .target
            .to_str()
            .is_none_or(|path| path.len() > 65_536)
        || output
            .target
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::RootDir))
        || output.target.file_name().and_then(|name| name.to_str())
            != Some(&format!("{}.{}.json", output.recording, output.engine))
        || Sha256Digest(Sha256::digest(&output.bytes).into()) != output.digest
    {
        return Err(invalid());
    }
    #[derive(serde::Deserialize)]
    struct Identity {
        engine: engine::EngineMetadata,
        local: bool,
    }
    let identity: Identity = serde_json::from_slice(&output.bytes).map_err(|_| invalid())?;
    engine::decode(
        &output.bytes,
        &ExpectedEngine {
            name: output.engine.clone(),
            family: identity.engine.family,
            local: identity.local,
        },
        output.duration,
    )
    .map(|decoded| decoded.document)
    .map_err(|_| invalid())
}

pub(super) fn check_batch(
    delta: &WorkDelta,
    previous: impl Fn(&Path) -> Result<(Option<Sha256Digest>, bool), LedgerError>,
) -> Result<(), LedgerError> {
    if delta.engine_outputs.is_empty() {
        return Ok(());
    }
    let Some(TranscriptionChange::Accept(accepted)) = &delta.transcription else {
        return Err(conflict());
    };
    if delta.engine_outputs.len() != 1 + usize::from(accepted.checker_engine.is_some()) {
        return Err(conflict());
    }
    let mut names = std::collections::BTreeSet::new();
    let mut record_found = false;
    for output in &delta.engine_outputs {
        let document = validate(output)?;
        if output.recording != accepted.recording
            || output.duration != accepted.duration
            || output.created_at != accepted.accepted_at
            || !names.insert(&output.engine)
        {
            return Err(conflict());
        }
        if output.engine == accepted.transcript.engine.name {
            if crate::transcript_document::from_document(document, output.language_source)
                != accepted.transcript
            {
                return Err(conflict());
            }
            record_found = true;
        } else if accepted.checker_engine.as_ref()
            != Some(&crate::transcript_document::from_metadata(document.engine))
        {
            return Err(conflict());
        }
        let (digest, pending) = previous(&output.target)?;
        if pending {
            return Err(conflict());
        }
        let entries: Vec<_> = delta
            .commit
            .publications
            .iter()
            .filter(|entry| entry.target == output.target)
            .collect();
        if entries.len() != 1
            || entries[0].intended != output.digest
            || entries[0].expected_previous != digest
            || entries[0].recorded_at != output.created_at
            || delta.publication_clears.contains(&output.target)
        {
            return Err(conflict());
        }
    }
    if !record_found {
        return Err(conflict());
    }
    Ok(())
}
