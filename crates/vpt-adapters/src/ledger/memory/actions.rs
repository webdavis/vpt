use super::{MemoryLedger, State, with};
use crate::ledger::actions::{check, conflict};
use vpt_application::ports::{ActionChange, ActionLedger, LedgerError, QueuedAction, WorkDelta};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SendState {
    Pending,
    Sent,
    Refused,
    Withdrawing,
}

#[derive(Clone)]
pub(super) struct Saved {
    action: QueuedAction,
    state: SendState,
    attempted: bool,
}

impl ActionLedger for MemoryLedger {
    fn pending_actions(&self) -> Result<Vec<QueuedAction>, LedgerError> {
        with(self, |state| in_state(state, SendState::Pending))
    }

    fn withdrawing_actions(&self) -> Result<Vec<QueuedAction>, LedgerError> {
        with(self, |state| in_state(state, SendState::Withdrawing))
    }
}

fn in_state(state: &State, wanted: SendState) -> Vec<QueuedAction> {
    let mut actions: Vec<_> = state
        .action_sends
        .iter()
        .filter(|saved| saved.state == wanted)
        .map(|saved| saved.action.clone())
        .collect();
    actions.sort_by(|a, b| (a.recording.as_str(), &a.id).cmp(&(b.recording.as_str(), &b.id)));
    actions
}

pub(super) fn apply(state: &mut State, delta: &WorkDelta) -> Result<(), LedgerError> {
    let Some(change) = &delta.actions else {
        return Ok(());
    };
    check(change)?;
    let sends = &mut state.action_sends;
    let matching = |sends: &mut Vec<Saved>, action: &QueuedAction, wanted: SendState| {
        sends
            .iter()
            .position(|saved| saved.action == *action && saved.state == wanted)
            .ok_or_else(conflict)
    };
    match change {
        ActionChange::Queue {
            recording,
            actions,
            dropped,
        } => {
            for action in actions {
                let same = |saved: &&mut Saved| {
                    saved.action.recording == action.recording && saved.action.id == action.id
                };
                match sends.iter_mut().find(same) {
                    Some(saved)
                        if saved.action.digest == action.digest
                            && saved.state != SendState::Withdrawing => {}
                    Some(saved) => {
                        saved.action = action.clone();
                        saved.state = SendState::Pending;
                    }
                    None => sends.push(Saved {
                        action: action.clone(),
                        state: SendState::Pending,
                        attempted: false,
                    }),
                }
            }
            sends.retain(|saved| {
                saved.attempted
                    || saved.action.recording != *recording
                    || !dropped.contains(&saved.action.id)
            });
            for saved in sends.iter_mut() {
                if saved.action.recording == *recording && dropped.contains(&saved.action.id) {
                    saved.state = SendState::Withdrawing;
                }
            }
        }
        ActionChange::Attempt(action) => {
            let index = matching(sends, action, SendState::Pending)?;
            sends[index].attempted = true;
        }
        ActionChange::Settle { action, refused } => {
            let index = matching(sends, action, SendState::Pending)?;
            sends[index].state = if *refused {
                SendState::Refused
            } else {
                SendState::Sent
            };
        }
        ActionChange::ResendRefused(recording) => {
            for saved in sends.iter_mut() {
                if saved.state == SendState::Refused
                    && recording
                        .as_ref()
                        .is_none_or(|only| *only == saved.action.recording)
                {
                    saved.state = SendState::Pending;
                }
            }
        }
        ActionChange::Withdrawn(action) => {
            let index = matching(sends, action, SendState::Withdrawing)?;
            sends.remove(index);
        }
    }
    Ok(())
}
