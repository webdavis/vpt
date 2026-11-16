mod dialogue;
use crate::spawn::{self, SpawnError, Status};
use crate::{digest_open, inspect_archived_audio};
pub use dialogue::converse;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use vpt_domain::digest::Sha256Digest;

const DEADLINE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechError {
    Missing(&'static str),
    Failed(&'static str, String),
}

pub struct Clip {
    pub path: PathBuf,
    pub digest: Sha256Digest,
    pub duration: f64,
    directory: TempDir,
}

impl Clip {
    pub fn directory(&self) -> &Path {
        self.directory.path()
    }
}

pub fn speak(text: &str, maximum: u64) -> Result<Clip, SpeechError> {
    let directory = private_directory()?;
    let path = directory.path().join("clip.m4a");
    run("say", &["-o", text_of(&path)?, text], &path)?;
    clip("say", directory, path, maximum)
}

fn failed(error: std::io::Error) -> SpeechError {
    SpeechError::Failed("say", error.kind().to_string())
}

fn private_directory() -> Result<TempDir, SpeechError> {
    crate::private_tempdir().map_err(failed)
}

fn text_of(path: &Path) -> Result<&str, SpeechError> {
    path.to_str()
        .ok_or_else(|| SpeechError::Failed("say", "path is not UTF-8".into()))
}

fn run(program: &'static str, arguments: &[&str], made: &Path) -> Result<(), SpeechError> {
    let argv: Vec<String> = std::iter::once(program)
        .chain(arguments.iter().copied())
        .map(str::to_owned)
        .collect();
    let outcome =
        spawn::run(&argv, b"", DEADLINE, spawn::OUTPUT_LIMIT).map_err(|error| match error {
            SpawnError::NotFound(_) => SpeechError::Missing(program),
            SpawnError::Io(detail) => SpeechError::Failed(program, detail),
        })?;
    let reason = match outcome.status {
        Status::Exited(0) if made.is_file() => return Ok(()),
        Status::Exited(0) => "wrote no audio file".into(),
        Status::Exited(code) => format!("exited {code}"),
        status => format!("{status:?}"),
    };
    Err(SpeechError::Failed(program, reason))
}

fn clip(
    program: &'static str,
    directory: TempDir,
    path: PathBuf,
    maximum: u64,
) -> Result<Clip, SpeechError> {
    let digest = digest_open(&mut std::fs::File::open(&path).map_err(failed)?).map_err(failed)?;
    let duration = inspect_archived_audio(directory.path(), &path, digest, maximum)
        .map_err(|error| SpeechError::Failed(program, format!("unreadable audio: {error:?}")))?;
    Ok(Clip {
        path,
        digest,
        duration,
        directory,
    })
}
