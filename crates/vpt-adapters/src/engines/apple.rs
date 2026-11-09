use super::{decode, process};
use crate::{HelperClient, HelperError, Status};
use std::path::PathBuf;
use vpt_application::engines::{EngineConfig, EngineKind};
use vpt_application::ports::{Engine, EngineFailure, EngineOutput, EngineRequest};

pub struct AppleEngine {
    path: PathBuf,
    helper: HelperClient,
}

impl AppleEngine {
    pub fn new(path: PathBuf) -> Self {
        Self {
            helper: HelperClient::new(path.clone()),
            path,
        }
    }
}

impl Engine for AppleEngine {
    fn transcribe(&self, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure> {
        if !matches!(request.engine.kind, EngineKind::Apple) {
            return Err(EngineFailure::InvalidArguments);
        }
        if !request.duration.is_finite() || request.duration < 0.0 {
            return Err(EngineFailure::InvalidAudioDuration);
        }
        let path = self.path.to_str().ok_or(EngineFailure::InvalidArguments)?;
        let audio = request
            .audio
            .to_str()
            .ok_or(EngineFailure::InvalidArguments)?;
        self.helper.version().map_err(|error| match error {
            HelperError::Absent => EngineFailure::Unavailable,
            HelperError::MajorMismatch { found } => EngineFailure::HelperVersion(found),
            HelperError::Failed(_) => EngineFailure::Io,
            HelperError::Unknown(detail) if detail == "interrupted" => EngineFailure::Interrupted,
            HelperError::Unknown(detail) if detail == "the helper exceeded its deadline" => {
                EngineFailure::DeadlineExceeded
            }
            HelperError::Unknown(_) => EngineFailure::InvalidDocument,
        })?;
        let argv = vec![
            path.into(),
            "transcribe".into(),
            audio.into(),
            "--locale".into(),
            request.language.into(),
        ];
        let outcome = process::run(&argv, request.timeout)?;
        if outcome.status == Status::Exited(1) {
            return Err(speech_failure(&outcome.stderr));
        }
        process::success(outcome.status)?;
        let mut output = normalize(outcome.stdout, request)?;
        output.diagnostics.extend(self.helper.take_diagnostics());
        Ok(output)
    }
}

fn normalize(bytes: Vec<u8>, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure> {
    if request.engine.name == "apple" {
        return decode::document(bytes, request);
    }
    let canonical = EngineConfig {
        name: "apple".into(),
        kind: EngineKind::Apple,
    };
    let validation = EngineRequest {
        engine: &canonical,
        ..*request
    };
    let mut output = decode::document(bytes, &validation)?;
    let mut document: serde_json::Value =
        serde_json::from_slice(&output.raw_document).map_err(|_| EngineFailure::InvalidDocument)?;
    document["engine"]["name"] = request.engine.name.clone().into();
    output.raw_document =
        serde_json::to_vec(&document).map_err(|_| EngineFailure::InvalidDocument)?;
    if output.raw_document.len() > super::DOCUMENT_LIMIT {
        return Err(EngineFailure::OutputLimit);
    }
    output.transcript.engine.name = request.engine.name.clone();
    Ok(output)
}

fn speech_failure(stderr: &[u8]) -> EngineFailure {
    match stderr {
        b"vpt-macos: Apple speech requires macOS 26 or later\n" => {
            EngineFailure::UnsupportedPlatform
        }
        b"vpt-macos: Apple SpeechTranscriber is unavailable on this Mac\n" => {
            EngineFailure::Unavailable
        }
        b"vpt-macos: Apple speech does not support the requested locale\n" => {
            EngineFailure::UnsupportedLanguage
        }
        b"vpt-macos: Apple speech assets are not installed for the requested locale\n" => {
            EngineFailure::MissingSpeechAssets
        }
        _ => EngineFailure::Exited(1),
    }
}

#[cfg(test)]
mod tests;
