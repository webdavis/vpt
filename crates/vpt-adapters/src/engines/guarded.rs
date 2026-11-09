use super::{EngineAdapters, audio::inspect_archived_audio, workspace::Workspace};
use std::path::PathBuf;
use vpt_application::ports::{Engine, EngineFailure, EngineOutput, EngineRequest};
use vpt_domain::digest::Sha256Digest;

pub struct GuardedEngines {
    inner: EngineAdapters,
    digest: Sha256Digest,
    maximum: u64,
}

impl GuardedEngines {
    pub fn new(helper: PathBuf, digest: Sha256Digest, maximum: u64) -> Self {
        Self {
            inner: EngineAdapters::new(helper),
            digest,
            maximum,
        }
    }
}

impl Engine for GuardedEngines {
    fn transcribe(&self, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure> {
        let workspace = Workspace::new_limited(request.out_dir, request.audio, self.maximum)?;
        let result = (|| {
            let audio = workspace.audio();
            let duration =
                inspect_archived_audio(workspace.path(), &audio, self.digest, self.maximum)
                    .map_err(|_| EngineFailure::Io)?;
            if duration != request.duration {
                return Err(EngineFailure::InvalidAudioDuration);
            }
            self.inner.transcribe(&EngineRequest {
                engine: request.engine,
                audio: &audio,
                out_dir: workspace.path(),
                language: request.language,
                duration,
                timeout: request.timeout,
            })
        })();
        workspace.close()?;
        result
    }
}
