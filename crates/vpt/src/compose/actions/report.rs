use super::{Counts, Delivery};
use std::path::PathBuf;
use vpt_adapters::SystemClock;
use vpt_application::ports::*;
use vpt_domain::{
    history::{Event, EventDetail, EventKind, EventState},
    identity::RecordingId,
    run::HistoryFact,
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn commit<E: EventSink>(
    delivery: &mut Delivery<'_, E>,
    recording: &RecordingId,
    note: Option<PathBuf>,
    counts: Counts,
) -> Result<(), Box<ErrorDocument>> {
    if counts.is_empty() {
        return Ok(());
    }
    let at = SystemClock.now();
    let mut event = Event::recording_done(EventKind::ActionsSent, recording.clone(), note, at);
    event.detail = EventDetail::Actions {
        sent: counts.sent,
        failed: counts.failed,
        refused: counts.refused,
        withdrawn: counts.withdrawn,
    };
    if counts.failed + counts.refused > 0 {
        event.state = EventState::Failed;
    }
    crate::compose::history::commit(
        delivery.ledger.commit_step(
            delivery.run,
            &WorkDelta::default(),
            &[(at, HistoryFact::Event(event))],
        ),
        delivery.sink,
        delivery.warnings,
    )
    .map(|_| ())
}
