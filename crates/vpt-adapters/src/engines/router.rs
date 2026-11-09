use super::{AppleEngine, CommandEngine, WhisplyEngine};
use std::path::PathBuf;
use vpt_application::engines::EngineKind;
use vpt_application::ports::{Engine, EngineFailure, EngineOutput, EngineRequest};

pub struct EngineAdapters {
    apple: AppleEngine,
}

impl EngineAdapters {
    pub fn new(helper_path: PathBuf) -> Self {
        Self {
            apple: AppleEngine::new(helper_path),
        }
    }
}

impl Engine for EngineAdapters {
    fn transcribe(&self, request: &EngineRequest<'_>) -> Result<EngineOutput, EngineFailure> {
        match request.engine.kind {
            EngineKind::Apple => self.apple.transcribe(request),
            EngineKind::Whisply { .. } => WhisplyEngine.transcribe(request),
            EngineKind::Command { .. } => CommandEngine.transcribe(request),
        }
    }
}

#[cfg(test)]
mod tests;
