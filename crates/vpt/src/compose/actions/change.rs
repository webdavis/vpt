use super::Delivery;
use crate::compose::history::commit;
use vpt_adapters::SystemClock;
use vpt_application::ports::*;
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, WorkOutcome},
};
use vpt_protocol::error::ErrorDocument;

impl<E: EventSink> Delivery<'_, E> {
    pub(super) fn commit_change(
        &mut self,
        recording: Option<&RecordingId>,
        change: ActionChange,
    ) -> Result<(), Box<ErrorDocument>> {
        let delta = WorkDelta {
            actions: Some(change),
            ..Default::default()
        };
        let fact = HistoryFact::WorkStep {
            recording: recording.cloned(),
            outcome: WorkOutcome::Completed,
        };
        commit(
            self.ledger
                .commit_step(self.run, &delta, &[(SystemClock.now(), fact)]),
            self.sink,
            self.warnings,
        )
        .map(|_| ())
    }
}
