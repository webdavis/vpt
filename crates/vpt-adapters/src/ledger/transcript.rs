mod checker;
mod encode;
mod history;
use crate::transcript_document::{from_document, to_document};
pub(super) use history::check_history;
use serde::{Deserialize, Serialize};
use vpt_application::ports::{AcceptedTranscript, LedgerError};
use vpt_domain::{identity::RecordingId, time::UtcInstant, transcript::LanguageSource};
use vpt_protocol::engine::{self, ExpectedEngine};

pub(super) const MAX_BODY: usize = 3 * 16_777_216;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checker_engine: Option<engine::EngineMetadata>,
    version: u32,
    requested_language: String,
    duration: f64,
    language_source: Source,
    lexical_classification: bool,
    from_checker: bool,
    document: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Source {
    Reported,
    Requested,
}

pub(super) fn encode(accepted: &AcceptedTranscript) -> Result<String, LedgerError> {
    if accepted.requested_language.len() > 65_536 {
        return Err(corrupt());
    }
    checker::check_accepted(accepted)?;
    encode::check_shape(&accepted.transcript)?;
    let document = encode::json(&to_document(&accepted.transcript), 16_777_216)?;
    let saved = Saved {
        checker_engine: accepted
            .checker_engine
            .as_ref()
            .map(crate::transcript_document::to_metadata),
        version: 1,
        requested_language: accepted.requested_language.clone(),
        duration: accepted.duration,
        language_source: match accepted.transcript.language_source {
            LanguageSource::Reported => Source::Reported,
            LanguageSource::Requested => Source::Requested,
        },
        lexical_classification: accepted.lexical_classification,
        from_checker: accepted.from_checker,
        document,
    };
    validate(&saved, &accepted.transcript.engine.name)?;
    encode::json(&saved, MAX_BODY)
}

pub(super) fn decode(
    recording: RecordingId,
    engine: &str,
    body: &str,
    accepted_at: UtcInstant,
) -> Result<AcceptedTranscript, LedgerError> {
    if body.len() > MAX_BODY {
        return Err(corrupt());
    }
    let saved: Saved = serde_json::from_str(body).map_err(|_| corrupt())?;
    let document = validate(&saved, engine)?;
    Ok(AcceptedTranscript {
        checker_engine: saved
            .checker_engine
            .map(crate::transcript_document::from_metadata),
        recording,
        accepted_at,
        requested_language: saved.requested_language,
        duration: saved.duration,
        lexical_classification: saved.lexical_classification,
        from_checker: saved.from_checker,
        uncertainty: Default::default(),
        transcript: from_document(
            document,
            match saved.language_source {
                Source::Reported => LanguageSource::Reported,
                Source::Requested => LanguageSource::Requested,
            },
        ),
    })
}

fn validate(saved: &Saved, engine_name: &str) -> Result<engine::EngineDocument, LedgerError> {
    checker::validate(
        saved.checker_engine.as_ref(),
        engine_name,
        saved.from_checker,
    )?;
    if saved.version != 1
        || saved.document.len() > 16_777_216
        || saved.requested_language.len() > 65_536
        || saved.requested_language.trim().is_empty()
        || saved.requested_language.chars().any(char::is_control)
    {
        return Err(corrupt());
    }
    #[derive(Deserialize)]
    struct Identity {
        engine: engine::EngineMetadata,
        local: bool,
    }
    let identity: Identity = serde_json::from_str(&saved.document).map_err(|_| corrupt())?;
    let expected = ExpectedEngine {
        name: engine_name.into(),
        family: identity.engine.family,
        local: identity.local,
    };
    engine::decode(saved.document.as_bytes(), &expected, saved.duration)
        .map(|decoded| decoded.document)
        .map_err(|_| corrupt())
}

pub(super) fn corrupt() -> LedgerError {
    LedgerError::Corrupt("invalid accepted transcript".into())
}

#[cfg(test)]
mod tests;
