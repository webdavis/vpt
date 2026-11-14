use super::{Counts, Delivery};
use vpt_adapters::{WithdrawOutcome, withdraw_action};
use vpt_application::ports::*;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

impl<E: EventSink> Delivery<'_, E> {
    pub(super) fn withdraw(
        &mut self,
        recording: &RecordingId,
        withdrawing: &[QueuedAction],
        counts: &mut Counts,
    ) -> Result<(), Box<ErrorDocument>> {
        let Some(withdrawn) = counts.withdrawn.as_mut() else {
            return Ok(());
        };
        for action in withdrawing {
            match withdraw_action(self.settings, recording, &action.id) {
                WithdrawOutcome::Failed => {
                    self.warnings.insert(SinkWarning::WithdrawFailed);
                    counts.failed += 1;
                    continue;
                }
                WithdrawOutcome::Withdrawn => *withdrawn += 1,
                WithdrawOutcome::Refused => {
                    self.warnings.insert(SinkWarning::WithdrawRefused);
                    counts.refused += 1;
                }
            }
            self.commit_change(Some(recording), ActionChange::Withdrawn(action.clone()))?;
        }
        Ok(())
    }
}
