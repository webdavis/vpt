use vpt_application::ports::{EngineFailure, EngineOutput, EngineRequest};
use vpt_domain::transcript::LanguageSource;
use vpt_protocol::engine::{self, EngineDecodeError, ExpectedEngine};

pub(super) fn document(
    bytes: Vec<u8>,
    request: &EngineRequest<'_>,
) -> Result<EngineOutput, EngineFailure> {
    let expected = ExpectedEngine {
        name: request.engine.name.clone(),
        family: request.engine.family().into(),
        local: request.engine.local(),
    };
    let decoded = engine::decode(&bytes, &expected, request.duration).map_err(failure)?;
    let document = decoded.document;
    Ok(EngineOutput {
        raw_document: bytes,
        diagnostics: decoded.diagnostics,
        transcript: crate::transcript_document::from_document(document, LanguageSource::Reported),
    })
}

pub(super) fn failure(error: EngineDecodeError) -> EngineFailure {
    match error {
        EngineDecodeError::UnsupportedVersion(version) => {
            EngineFailure::UnsupportedVersion(version)
        }
        EngineDecodeError::Limit(vpt_protocol::LimitViolation::Syntax(_))
        | EngineDecodeError::Invalid(_) => EngineFailure::InvalidDocument,
        EngineDecodeError::Limit(_) => EngineFailure::OutputLimit,
    }
}
