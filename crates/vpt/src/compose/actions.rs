mod change;
mod counts;
mod queue;
mod report;
mod resend;
use super::errors::ledger_error;
pub(crate) use counts::Counts;
pub(crate) use queue::queue;
use queue::queued;
pub(crate) use resend::require_command;
use std::collections::BTreeSet;
use vpt_adapters::{ActionSend, SendOutcome, SqliteLedger, note_link, send_action};
use vpt_application::{Settings, ports::*};
use vpt_domain::{history::RunId, identity::RecordingId};
use vpt_protocol::error::ErrorDocument;

pub(crate) struct Delivery<'a, E> {
    pub ledger: &'a SqliteLedger,
    pub settings: &'a Settings,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
    pub run: RunId,
}

impl<E: EventSink> Delivery<'_, E> {
    /// Sends the queued actions of one recording, or of every recording, and counts the outcomes.
    /// Resending refused actions first makes that scope's refused actions pending again.
    pub(crate) fn send(
        &mut self,
        only: Option<&RecordingId>,
        resend_refused: bool,
    ) -> Result<Counts, Box<ErrorDocument>> {
        let mut total = Counts::default();
        if self.settings.actions_command.is_empty() {
            return Ok(total);
        }
        if resend_refused {
            resend::refused(self, only)?;
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
            self.commit_change(
                Some(recording),
                ActionChange::Settle {
                    action: queued,
                    refused,
                },
            )?;
        }
        report::commit(self, recording, note, counts)?;
        Ok(counts)
    }
}

fn error(error: LedgerError) -> Box<ErrorDocument> {
    Box::new(ledger_error(error))
}
