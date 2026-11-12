use super::LedgerError;
use vpt_domain::identity::RecordingId;

pub trait ActionLedger {
    fn pending_actions(&self) -> Result<Vec<QueuedAction>, LedgerError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedAction {
    pub recording: RecordingId,
    pub id: String,
    pub digest: String,
}

/// Queuing an id already saved with the same digest keeps its state; a new digest makes it
/// pending again. Settling marks a pending action whose digest still matches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionChange {
    Queue(Vec<QueuedAction>),
    Settle { action: QueuedAction, refused: bool },
}
