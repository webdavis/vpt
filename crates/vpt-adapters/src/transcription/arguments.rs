use super::language;
use std::path::Path;
use vpt_application::engines::{EngineConfig, EngineKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub argv: Vec<String>,
    pub requested_language: String,
    pub backend_language: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationError {
    InvalidLanguage,
    UnsupportedLanguage,
    InvalidPath,
    EmptyCommand,
}

pub fn invocation(
    engine: &EngineConfig,
    helper: &Path,
    audio: &Path,
    output: &Path,
    language: &str,
) -> Result<Invocation, InvocationError> {
    language::validate(language)?;
    let audio = path_text(audio)?;
    let output = path_text(output)?;
    let mut backend_language = language.to_owned();
    let argv = match &engine.kind {
        EngineKind::Apple => vec![
            path_text(helper)?.into(),
            "transcribe".into(),
            audio.into(),
            "--locale".into(),
            language.into(),
        ],
        EngineKind::Whisply {
            command,
            device,
            model,
            ..
        } => {
            backend_language = language::whisply(language, model)?;
            let mut argv = command.clone();
            argv.extend(
                [
                    "run",
                    "-f",
                    audio,
                    "-o",
                    output,
                    "-d",
                    device,
                    "-m",
                    model,
                    "-l",
                    &backend_language,
                    "-e",
                    "json",
                ]
                .into_iter()
                .map(str::to_owned),
            );
            if command.is_empty() {
                return Err(InvocationError::EmptyCommand);
            }
            argv
        }
        EngineKind::Command { command, .. } => command
            .iter()
            .map(|arg| substitute(arg, audio, output, language))
            .collect(),
    };
    if argv.first().is_none_or(|program| program.is_empty())
        || argv.iter().any(|arg| arg.contains('\0'))
    {
        return Err(InvocationError::EmptyCommand);
    }
    Ok(Invocation {
        argv,
        requested_language: language.into(),
        backend_language,
    })
}

fn path_text(path: &Path) -> Result<&str, InvocationError> {
    path.to_str()
        .filter(|text| !text.is_empty() && !text.contains('\0'))
        .ok_or(InvocationError::InvalidPath)
}

fn substitute(template: &str, audio: &str, output: &str, language: &str) -> String {
    let mut result = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        let replacement = [
            ("{audio}", audio),
            ("{out_dir}", output),
            ("{language}", language),
        ]
        .into_iter()
        .find(|(token, _)| rest.starts_with(token));
        if let Some((token, value)) = replacement {
            result.push_str(value);
            rest = &rest[token.len()..];
        } else {
            result.push('{');
            rest = &rest[1..];
        }
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests;
