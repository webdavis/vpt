use super::Delivery;
use crate::compose::history::commit;
use vpt_adapters::SystemClock;
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    identity::RecordingId,
    run::{HistoryFact, WorkOutcome},
};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn require_command(settings: &Settings) -> Result<(), Box<ErrorDocument>> {
    if settings.actions_command.is_empty() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "--resend-refused requires [actions] command",
        )));
    }
    Ok(())
}

pub(super) fn refused<E: EventSink>(
    delivery: &mut Delivery<'_, E>,
    only: Option<&RecordingId>,
) -> Result<(), Box<ErrorDocument>> {
    let delta = WorkDelta {
        actions: Some(ActionChange::ResendRefused(only.cloned())),
        ..Default::default()
    };
    let fact = HistoryFact::WorkStep {
        recording: only.cloned(),
        outcome: WorkOutcome::Completed,
    };
    commit(
        delivery
            .ledger
            .commit_step(delivery.run, &delta, &[(SystemClock.now(), fact)]),
        delivery.sink,
        delivery.warnings,
    )
    .map(|_| ())
}
