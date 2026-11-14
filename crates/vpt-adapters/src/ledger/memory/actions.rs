use super::{MemoryLedger, State, with};
use crate::ledger::actions::{check, conflict};
use vpt_application::ports::{ActionChange, ActionLedger, LedgerError, QueuedAction, WorkDelta};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SendState {
    Pending,
    Sent,
    Refused,
}

impl ActionLedger for MemoryLedger {
    fn pending_actions(&self) -> Result<Vec<QueuedAction>, LedgerError> {
        with(self, |state| {
            let mut pending: Vec<_> = state
                .action_sends
                .iter()
                .filter(|(_, send)| *send == SendState::Pending)
                .map(|(action, _)| action.clone())
                .collect();
            pending
                .sort_by(|a, b| (a.recording.as_str(), &a.id).cmp(&(b.recording.as_str(), &b.id)));
            pending
        })
    }
}

pub(super) fn apply(state: &mut State, delta: &WorkDelta) -> Result<(), LedgerError> {
    let Some(change) = &delta.actions else {
        return Ok(());
    };
    check(change)?;
    let same = |saved: &QueuedAction, action: &QueuedAction| {
        saved.recording == action.recording && saved.id == action.id
    };
    match change {
        ActionChange::Queue(actions) => {
            for action in actions {
                match state
                    .action_sends
                    .iter_mut()
                    .find(|(saved, _)| same(saved, action))
                {
                    Some((saved, _)) if saved.digest == action.digest => {}
                    Some(entry) => *entry = (action.clone(), SendState::Pending),
                    None => state
                        .action_sends
                        .push((action.clone(), SendState::Pending)),
                }
            }
        }
        ActionChange::Settle { action, refused } => {
            let (_, send) = state
                .action_sends
                .iter_mut()
                .find(|(saved, send)| *saved == *action && *send == SendState::Pending)
                .ok_or_else(conflict)?;
            *send = if *refused {
                SendState::Refused
            } else {
                SendState::Sent
            };
        }
        ActionChange::ResendRefused(recording) => {
            for (saved, send) in &mut state.action_sends {
                if *send == SendState::Refused
                    && recording
                        .as_ref()
                        .is_none_or(|only| *only == saved.recording)
                {
                    *send = SendState::Pending;
                }
            }
        }
    }
    Ok(())
}
