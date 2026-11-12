use super::{DOCUMENT_LIMIT, decode, process};
use vpt_application::engines::EngineKind;
use vpt_application::ports::{Engine, EngineFailure, EngineOutput, EngineRequest};
use vpt_domain::transcript::LanguageSource;
mod language;
use super::workspace;

pub struct WhisplyEngine;

impl Engine for WhisplyEngine {
    fn transcribe(&self, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure> {
        if !request.duration.is_finite() || request.duration < 0.0 {
            return Err(EngineFailure::InvalidAudioDuration);
        }
        let EngineKind::Whisply {
            command,
            device,
            model,
            hf_token,
        } = &request.engine.kind
        else {
            return Err(EngineFailure::InvalidArguments);
        };
        if command
            .first()
            .is_none_or(|program| program.trim().is_empty())
            || command.iter().any(|argument| argument.contains('\0'))
        {
            return Err(EngineFailure::InvalidArguments);
        }
        let language = language::base(request.language, model)?;
        let workspace = workspace::Workspace::new(request.out_dir, request.audio)?;
        let result = execute(
            request,
            command,
            device,
            model,
            language,
            hf_token.as_deref(),
            &workspace,
        );
        workspace.close()?;
        result
    }
}

fn execute(
    request: &EngineRequest<'_>,
    command: &[String],
    device: &str,
    model: &str,
    language: &str,
    hf_token: Option<&str>,
    workspace: &workspace::Workspace,
) -> Result<EngineOutput, EngineFailure> {
    let mut argv = command.to_vec();
    let audio = workspace.audio();
    let audio = audio.to_str().ok_or(EngineFailure::InvalidArguments)?;
    let out_dir = workspace
        .path()
        .to_str()
        .ok_or(EngineFailure::InvalidArguments)?;
    argv.extend(
        [
            "run", "-f", audio, "-o", out_dir, "-d", device, "-m", model, "-l", language, "-e",
            "json",
        ]
        .map(str::to_owned),
    );
    let env: Vec<_> = hf_token
        .map(|token| ("HF_TOKEN", token))
        .into_iter()
        .collect();
    if !env.is_empty() {
        argv.push("-a".into());
    }
    workspace.revalidate()?;
    let outcome = process::run_with_limit(&argv, request.timeout, crate::OUTPUT_LIMIT, &env)?;
    process::success(outcome.status).map_err(|failure| match failure {
        EngineFailure::Exited(code) if hf_token.is_some() => EngineFailure::SpeakerAnnotation(code),
        failure => failure,
    })?;
    let decoded = vpt_protocol::whisply::decode(
        &workspace.read_document()?,
        &vpt_protocol::whisply::WhisplyRequest {
            name: &request.engine.name,
            language: request.language,
            backend_language: language,
            device,
            duration: request.duration,
        },
    )
    .map_err(decode::failure)?;
    let bytes =
        serde_json::to_vec(&decoded.document).map_err(|_| EngineFailure::InvalidDocument)?;
    if bytes.len() > DOCUMENT_LIMIT {
        return Err(EngineFailure::OutputLimit);
    }
    let mut output = decode::document(bytes, request)?;
    output.transcript.language_source = LanguageSource::Requested;
    output.diagnostics.extend(decoded.diagnostics);
    Ok(output)
}

#[cfg(test)]
mod tests;
