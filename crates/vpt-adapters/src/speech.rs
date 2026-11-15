use crate::spawn::{self, SpawnError, Status};
use crate::{digest_open, inspect_archived_audio};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use vpt_domain::digest::Sha256Digest;

const DEADLINE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechError {
    Missing,
    Failed(String),
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
    let failed = |error: std::io::Error| SpeechError::Failed(error.kind().to_string());
    let directory = tempfile::Builder::new()
        .prefix("vpt-test-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(std::env::temp_dir().canonicalize().map_err(failed)?)
        .map_err(failed)?;
    let path = directory.path().join("clip.m4a");
    let output = path
        .to_str()
        .ok_or_else(|| SpeechError::Failed("path is not UTF-8".into()))?;
    let argv = ["say", "-o", output, text].map(str::to_owned);
    let outcome =
        spawn::run(&argv, b"", DEADLINE, spawn::OUTPUT_LIMIT).map_err(|error| match error {
            SpawnError::NotFound(_) => SpeechError::Missing,
            SpawnError::Io(detail) => SpeechError::Failed(detail),
        })?;
    match outcome.status {
        Status::Exited(0) if path.is_file() => {}
        Status::Exited(0) => return Err(SpeechError::Failed("wrote no audio file".into())),
        Status::Exited(code) => return Err(SpeechError::Failed(format!("exited {code}"))),
        status => return Err(SpeechError::Failed(format!("{status:?}"))),
    }
    let digest = digest_open(&mut std::fs::File::open(&path).map_err(failed)?).map_err(failed)?;
    let duration = inspect_archived_audio(directory.path(), &path, digest, maximum)
        .map_err(|error| SpeechError::Failed(format!("unreadable audio: {error:?}")))?;
    Ok(Clip {
        path,
        digest,
        duration,
        directory,
    })
}
