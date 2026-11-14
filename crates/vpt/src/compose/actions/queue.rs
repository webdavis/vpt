use vpt_adapters::action_digest;
use vpt_application::{
    Settings,
    ports::{ActionChange, QueuedAction},
};
use vpt_domain::{
    identity::RecordingId,
    proposal::{CheckedProposal, action_ids},
};

pub(crate) fn queue(
    settings: &Settings,
    recording: &RecordingId,
    proposal: &CheckedProposal,
) -> Option<ActionChange> {
    (!settings.actions_command.is_empty()).then(|| ActionChange::Queue(queued(recording, proposal)))
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
