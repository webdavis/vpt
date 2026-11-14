mod report;
use super::{errors::ledger_error, history::commit};
use std::collections::BTreeSet;
use vpt_adapters::{
    ActionSend, SendOutcome, SqliteLedger, SystemClock, action_digest, note_link, send_action,
};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    history::RunId,
    identity::RecordingId,
    proposal::{CheckedProposal, action_ids},
    run::{HistoryFact, WorkOutcome},
};
use vpt_protocol::error::ErrorDocument;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub sent: u32,
    pub failed: u32,
    pub refused: u32,
}

impl Counts {
    pub(crate) fn json(self) -> serde_json::Value {
        serde_json::json!({"sent": self.sent, "failed": self.failed, "refused": self.refused})
    }

    pub(crate) fn parse(value: &serde_json::Value) -> Self {
        let field = |name: &str| {
            value[name]
                .as_u64()
                .and_then(|count| u32::try_from(count).ok())
                .unwrap_or_default()
        };
        Self {
            sent: field("sent"),
            failed: field("failed"),
            refused: field("refused"),
        }
    }
}

impl std::ops::AddAssign for Counts {
    fn add_assign(&mut self, other: Self) {
        self.sent += other.sent;
        self.failed += other.failed;
        self.refused += other.refused;
    }
}

pub(crate) fn queue(
    settings: &Settings,
    recording: &RecordingId,
    proposal: &CheckedProposal,
) -> Option<ActionChange> {
    (!settings.actions_command.is_empty()).then(|| ActionChange::Queue(queued(recording, proposal)))
}

fn queued(recording: &RecordingId, proposal: &CheckedProposal) -> Vec<QueuedAction> {
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

pub(crate) struct Delivery<'a, E> {
    pub ledger: &'a SqliteLedger,
    pub settings: &'a Settings,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
    pub run: RunId,
}

impl<E: EventSink> Delivery<'_, E> {
    /// Sends the queued actions of one recording, or of every recording, and counts the outcomes.
    pub(crate) fn send(
        &mut self,
        only: Option<&RecordingId>,
    ) -> Result<Counts, Box<ErrorDocument>> {
        let mut total = Counts::default();
        if self.settings.actions_command.is_empty() {
            return Ok(total);
        }
        let mut pending = self.ledger.pending_actions().map_err(error)?;
        pending.retain(|queued| only.is_none_or(|only| *only == queued.recording));
        while let Some(first) = pending.first() {
            let recording = first.recording.clone();
            let (mine, rest) = pending
                .into_iter()
                .partition(|queued| queued.recording == recording);
            pending = rest;
            total += self.recording(&recording, &mine)?;
        }
        Ok(total)
    }

    fn recording(
        &mut self,
        recording: &RecordingId,
        pending: &[QueuedAction],
    ) -> Result<Counts, Box<ErrorDocument>> {
        let mut counts = Counts::default();
        let Some(accepted) = self.ledger.accepted_proposal(recording).map_err(error)? else {
            return Ok(counts);
        };
        let note = self
            .ledger
            .note_path(recording, NoteStage::Analysis)
            .map_err(error)?;
        let link = note
            .as_ref()
            .map(|note| note_link(note, self.settings.note.profile))
            .unwrap_or_default();
        let path = note
            .as_ref()
            .map(|note| note.to_string_lossy().into_owned());
        let current = queued(recording, &accepted.proposal);
        for (queued, action) in current.into_iter().zip(&accepted.proposal.actions) {
            if !pending.contains(&queued) {
                continue;
            }
            let send = ActionSend {
                recording,
                action,
                id: &queued.id,
                note: path.as_deref().unwrap_or_default(),
                link: &link,
            };
            let outcome = send_action(self.settings, &send);
            let refused = match outcome {
                SendOutcome::Failed => {
                    self.warnings.insert(SinkWarning::ActionFailed);
                    counts.failed += 1;
                    continue;
                }
                SendOutcome::Sent => {
                    counts.sent += 1;
                    false
                }
                SendOutcome::Refused => {
                    self.warnings.insert(SinkWarning::ActionRefused);
                    counts.refused += 1;
                    true
                }
            };
            let delta = WorkDelta {
                actions: Some(ActionChange::Settle {
                    action: queued,
                    refused,
                }),
                ..Default::default()
            };
            let fact = HistoryFact::WorkStep {
                recording: Some(recording.clone()),
                outcome: WorkOutcome::Completed,
            };
            commit(
                self.ledger
                    .commit_step(self.run, &delta, &[(SystemClock.now(), fact)]),
                self.sink,
                self.warnings,
            )?;
        }
        report::commit(self, recording, note, counts)?;
        Ok(counts)
    }
}

fn error(error: LedgerError) -> Box<ErrorDocument> {
    Box::new(ledger_error(error))
}
