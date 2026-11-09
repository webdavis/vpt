use super::Context;
use crate::{cli::output::Outcome, compose::history::commit};
use vpt_adapters::SystemClock;
use vpt_application::ports::{Clock, EventSink, RunHistory, TranscriptionChange, WorkDelta};
use vpt_domain::history::RunId;
use vpt_domain::{
    history::{Event, EventCounts, EventDetail, EventKind, EventState},
    identity::RecordingId,
    run::{HistoryFact, RunOutcome, WorkOutcome},
};
use vpt_protocol::error::ErrorDocument;

pub(super) fn failure<E: EventSink>(
    context: &mut Context<'_, E>,
    run: RunId,
    id: &RecordingId,
    mut error: Box<ErrorDocument>,
    interrupted: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let at = SystemClock.now();
    let delta = WorkDelta {
        transcription: Some(TranscriptionChange::Fail(id.clone())),
        ..Default::default()
    };
    commit(
        context.ledger.commit_step(
            run,
            &delta,
            &[
                (
                    at,
                    HistoryFact::WorkStep {
                        recording: Some(id.clone()),
                        outcome: WorkOutcome::Failed,
                    },
                ),
                (
                    at,
                    HistoryFact::Event(Event {
                        kind: EventKind::TranscribeFailed,
                        state: EventState::Failed,
                        occasion: None,
                        recording: Some(id.clone()),
                        detail: EventDetail::None,
                        counts: EventCounts::default(),
                        paths: vec![],
                        occurred_at: at,
                    }),
                ),
                (
                    at,
                    HistoryFact::RunFinished(if interrupted {
                        RunOutcome::Interrupted
                    } else {
                        RunOutcome::Failed
                    }),
                ),
            ],
        ),
        context.sink,
        context.warnings,
    )?;
    error.ids.push(id.to_string());
    Err(error)
}
