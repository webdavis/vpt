use vpt_application::engines::SelectionError;
use vpt_application::transcribe::{TranscriptionError, save::SaveError};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn selection(error: SelectionError) -> Box<ErrorDocument> {
    Box::new(match error {
        SelectionError::SameFamily {
            main,
            checker,
            family,
        } => ErrorDocument::new(
            ErrorKind::Refused,
            format!(
                "main engine {main:?} and checker {checker:?} have the same model family {family:?}"
            ),
        )
        .rule("same_family"),
        SelectionError::InvalidLanguage => {
            ErrorDocument::new(ErrorKind::Usage, "invalid language tag")
        }
        SelectionError::MissingEngine { .. } => {
            ErrorDocument::new(ErrorKind::Config, "selected engine is not configured")
        }
    })
}

pub(crate) fn save(error: SaveError) -> Box<ErrorDocument> {
    Box::new(match error {
        SaveError::Ledger(error) => crate::compose::errors::ledger_error(error),
        SaveError::Store(error) => crate::compose::errors::store_error(error),
        SaveError::Modified(_) => ErrorDocument::new(
            ErrorKind::Refused,
            "engine output target was modified or is unowned",
        )
        .rule("target_modified"),
    })
}

pub(crate) fn engine(error: TranscriptionError) -> Box<ErrorDocument> {
    let interrupted = matches!(error, TranscriptionError::Interrupted(_));
    match error {
        TranscriptionError::Selection(error) => selection(error),
        TranscriptionError::InvalidAudioDuration => Box::new(
            ErrorDocument::new(ErrorKind::Refused, "invalid audio duration").rule("invalid_audio"),
        ),
        TranscriptionError::Failed(failures) | TranscriptionError::Interrupted(failures) => {
            let mut document = ErrorDocument::new(ErrorKind::Engine, "transcription failed")
                .diagnostics(
                    failures
                        .into_iter()
                        .map(|failure| attempt(&failure))
                        .collect(),
                );
            document.interrupted = interrupted;
            Box::new(document)
        }
    }
}

pub(crate) fn attempt(failure: &vpt_application::transcribe::AttemptFailure) -> String {
    use vpt_application::ports::EngineFailure;
    match &failure.cause {
        EngineFailure::WorkspaceCleanup(path) => format!(
            "{}: temporary engine files retained at {:?}",
            failure.engine, path
        ),
        EngineFailure::MissingSpeechAssets => format!(
            "{}: required Apple speech assets are not installed",
            failure.engine
        ),
        EngineFailure::Unavailable => format!(
            "{}: engine executable or helper is unavailable",
            failure.engine
        ),
        EngineFailure::SpeakerAnnotation(code) => format!(
            "{}: whisply exited {code} when asked for speaker annotation (-a); it may lack speaker annotation, or the Hugging Face account behind the token may not have accepted the terms of pyannote/speaker-diarization-3.1 and pyannote/segmentation-3.0",
            failure.engine
        ),
        cause => format!("{}: {cause:?}", failure.engine),
    }
}
