use crate::spawn::{self, SpawnError, Status};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

const DEADLINE: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    NotConfigured,
    Missing(String),
    Failed(String),
}

pub struct Extracted {
    pub path: PathBuf,
    _directory: TempDir,
}

pub fn extract_audio(command: &[String], input: &Path) -> Result<Extracted, ExtractError> {
    if command.is_empty() {
        return Err(ExtractError::NotConfigured);
    }
    let failed = |error: std::io::Error| ExtractError::Failed(error.kind().to_string());
    let directory = tempfile::Builder::new()
        .prefix("vpt-import-")
        .tempdir_in(std::env::temp_dir().canonicalize().map_err(failed)?)
        .map_err(failed)?;
    let path = directory.path().join("audio.m4a");
    let (Some(input), Some(output)) = (input.to_str(), path.to_str()) else {
        return Err(ExtractError::Failed("path is not UTF-8".into()));
    };
    let mut argv = command.to_vec();
    argv.extend(
        [
            "-nostdin",
            "-loglevel",
            "error",
            "-protocol_whitelist",
            "file",
            "-i",
            input,
            "-vn",
            "-map",
            "0:a:0",
            "-map_metadata",
            "0",
            "-c:a",
            "aac",
            output,
        ]
        .map(str::to_owned),
    );
    let outcome =
        spawn::run(&argv, b"", DEADLINE, spawn::OUTPUT_LIMIT).map_err(|error| match error {
            SpawnError::NotFound(program) => ExtractError::Missing(program),
            SpawnError::Io(detail) => ExtractError::Failed(detail),
        })?;
    match outcome.status {
        Status::Exited(0) if path.is_file() => Ok(Extracted {
            path,
            _directory: directory,
        }),
        Status::Exited(0) => Err(ExtractError::Failed("no audio was written".into())),
        status => Err(ExtractError::Failed(format!(
            "{status:?}: {}",
            String::from_utf8_lossy(&outcome.stderr).trim()
        ))),
    }
}
