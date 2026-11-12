use crate::ports::{StageState, StageStates};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextStage {
    Transcribe,
    Note,
    Analyze,
    Redact(crate::ports::NoteStage),
    Verify(crate::ports::NoteStage),
}
impl NextStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transcribe => "transcribe",
            Self::Note => "note",
            Self::Analyze => "analyze",
            Self::Redact(crate::ports::NoteStage::Transcript) => "redact_transcript",
            Self::Redact(crate::ports::NoteStage::Analysis) => "redact_analysis",
            Self::Verify(crate::ports::NoteStage::Transcript) => "verify_transcript",
            Self::Verify(crate::ports::NoteStage::Analysis) => "verify_analysis",
        }
    }
}

pub fn next(stages: StageStates) -> Option<NextStage> {
    let incomplete = |state| {
        matches!(
            state,
            StageState::Pending | StageState::Failed | StageState::Disabled
        )
    };
    if incomplete(stages.transcribe) {
        Some(NextStage::Transcribe)
    } else if incomplete(stages.note) {
        Some(NextStage::Note)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
