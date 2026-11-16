use super::RuntimeLedger;
use vpt_application::ports::{ActionLedger, LedgerError, QueuedAction, SendState};

impl ActionLedger for RuntimeLedger {
    fn actions_in(&self, state: SendState) -> Result<Vec<QueuedAction>, LedgerError> {
        match self {
            Self::Sqlite(ledger) => ledger.actions_in(state),
            Self::Empty(ledger) => ledger.actions_in(state),
        }
    }
}
