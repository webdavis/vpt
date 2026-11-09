mod audio;
mod guarded;
mod workspace;
pub use audio::{AudioError, inspect_archived_audio};
pub use guarded::GuardedEngines;
use vpt_application::engines::EngineKind;
use vpt_application::ports::{Engine, EngineFailure, EngineOutput, EngineRequest};

mod apple;
mod arguments;
pub use apple::AppleEngine;
mod decode;
mod process;
mod router;
pub use router::EngineAdapters;
mod whisply;
pub use whisply::WhisplyEngine;

const DOCUMENT_LIMIT: usize = 16_777_216;

pub struct CommandEngine;

impl Engine for CommandEngine {
    fn transcribe(&self, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure> {
        if !request.duration.is_finite() || request.duration < 0.0 {
            return Err(EngineFailure::InvalidAudioDuration);
        }
        let EngineKind::Command { command, .. } = &request.engine.kind else {
            return Err(EngineFailure::Unavailable);
        };
        let argv = arguments::substitute(command, request)?;
        let outcome = process::run(&argv, request.timeout)?;
        process::success(outcome.status)?;
        decode::document(outcome.stdout, request)
    }
}

#[cfg(test)]
mod tests;
