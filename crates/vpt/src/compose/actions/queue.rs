use vpt_adapters::action_digest;
use vpt_application::{
    Settings,
    ports::{ActionChange, QueuedAction},
};
use vpt_domain::{
    identity::RecordingId,
    proposal::{CheckedProposal, action_ids},
};

/// `dropped` names the ids the previous proposal held and this one does not, and only while the
/// withdraw command is set, so a drop published earlier is never withdrawn.
pub(crate) fn queue(
    settings: &Settings,
    recording: &RecordingId,
    previous: Option<&CheckedProposal>,
    proposal: &CheckedProposal,
) -> Option<ActionChange> {
    if settings.actions_command.is_empty() {
        return None;
    }
    let actions = queued(recording, proposal);
    let dropped = previous
        .filter(|_| !settings.actions_withdraw_command.is_empty())
        .map(|previous| action_ids(recording, &previous.actions))
        .unwrap_or_default()
        .into_iter()
        .filter(|id| actions.iter().all(|action| action.id != *id))
        .collect();
    Some(ActionChange::Queue {
        recording: recording.clone(),
        actions,
        dropped,
    })
}

pub(super) fn queued(recording: &RecordingId, proposal: &CheckedProposal) -> Vec<QueuedAction> {
    action_ids(recording, &proposal.actions)
        .into_iter()
        .zip(&proposal.actions)
        .map(|(id, action)| QueuedAction {
            recording: recording.clone(),
            id,
            digest: action_digest(action),
        })
        .collect()
}
