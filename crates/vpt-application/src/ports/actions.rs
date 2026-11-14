use super::LedgerError;
use vpt_domain::identity::RecordingId;

pub trait ActionLedger {
    fn pending_actions(&self) -> Result<Vec<QueuedAction>, LedgerError>;
    fn withdrawing_actions(&self) -> Result<Vec<QueuedAction>, LedgerError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedAction {
    pub recording: RecordingId,
    pub id: String,
    pub digest: String,
}

/// Queuing an id already saved with the same digest keeps its state; a new digest, or an id
/// awaiting withdrawal, makes it pending again. Each `dropped` id of the recording that a send was
/// attempted for awaits withdrawal; one never attempted is forgotten; saved ids in neither list
/// stay as they are. Attempting marks a pending action whose digest still matches as attempted,
/// before its command runs. Settling marks a pending action whose digest still matches. Resending
/// makes the refused actions of one recording, or of every recording, pending again. Withdrawn
/// forgets an action awaiting withdrawal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionChange {
    Queue {
        recording: RecordingId,
        actions: Vec<QueuedAction>,
        dropped: Vec<String>,
    },
    Attempt(QueuedAction),
    Settle {
        action: QueuedAction,
        refused: bool,
    },
    ResendRefused(Option<RecordingId>),
    Withdrawn(QueuedAction),
}
