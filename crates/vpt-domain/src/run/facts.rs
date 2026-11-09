use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryFact {
    BriefSaved {
        occasion: crate::occasion::OccasionId,
    },
    BriefPublished {
        occasion: crate::occasion::OccasionId,
    },
    OccasionSaved {
        occasion: crate::occasion::OccasionId,
    },
    SynthesisStateChanged {
        recording: RecordingId,
        from: crate::stage::StageState,
        to: crate::stage::StageState,
    },
    TagRejected {
        recording: RecordingId,
        index: u32,
        reason: crate::tags::Rejection,
    },
    FlagResolved {
        recording: RecordingId,
        flag: crate::digest::Sha256Digest,
        resolution: crate::review::ResolutionKind,
    },
    NoteMoved {
        recording: RecordingId,
        from: PathBuf,
        to: PathBuf,
    },
    RunStarted(RunKind),
    RunFinished(RunOutcome),
    WorkStep {
        recording: Option<RecordingId>,
        outcome: WorkOutcome,
    },
    EffectIntent {
        kind: EffectKind,
        reference: EffectReference,
    },
    EffectObserved {
        intent: EventId,
        outcome: EffectOutcome,
    },
    MissingRetentionHistory {
        intent_id: u64,
    },
    HookFailed {
        event: EventId,
        failure: HookFailureClass,
    },
    Event(Event),
    RecoveryProgress {
        evidence: RecoveryEvidence,
        baseline: RecoveryBaseline,
    },
    RecoverySummary {
        evidence: RecoveryEvidence,
        window: Option<FailureWindow>,
    },
}

impl HistoryFact {
    pub fn referenced_ids_are_nonzero(&self) -> bool {
        match self {
            Self::TagRejected { index, .. } => *index < 256,
            Self::RecoveryProgress { evidence, .. } | Self::RecoverySummary { evidence, .. } => {
                evidence.affected_runs.iter().all(|run| run.sequence() != 0)
            }
            Self::MissingRetentionHistory { intent_id } => *intent_id != 0,
            _ => true,
        }
    }
}
