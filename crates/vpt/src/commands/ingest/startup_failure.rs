use crate::compose::history::commit;
use std::collections::BTreeSet;
use vpt_adapters::{ContainedError, SystemClock};
use vpt_application::ports::{Clock, EventSink, RunHistory, SinkWarning, WorkDelta};
use vpt_domain::history::{
    Event, EventCounts, EventDetail, EventKind, EventState, SafeFailureClass,
};
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};
use vpt_protocol::error::ErrorDocument;

pub(crate) fn source<L: RunHistory, E: EventSink>(
    ledger: &L,
    sink: &mut E,
    cause: ContainedError,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Box<ErrorDocument> {
    let original = super::errors::source(cause);
    let result = save_failure(ledger, sink, warnings);
    result.err().unwrap_or_else(|| Box::new(original))
}

fn save_failure<L: RunHistory, E: EventSink>(
    ledger: &L,
    sink: &mut E,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<(), Box<ErrorDocument>> {
    let at = SystemClock.now();
    let receipt = commit(ledger.begin_run(RunKind::Ingest, at), sink, warnings)?;
    commit(
        ledger.commit_step(
            receipt.run,
            &WorkDelta::default(),
            &[
                (
                    at,
                    HistoryFact::Event(Event {
                        kind: EventKind::IngestFailed,
                        state: EventState::Failed,
                        occasion: None,
                        recording: None,
                        detail: EventDetail::Failure(SafeFailureClass::SourceUnavailable),
                        counts: EventCounts::default(),
                        paths: vec![],
                        occurred_at: at,
                    }),
                ),
                (at, HistoryFact::RunFinished(RunOutcome::Failed)),
            ],
        ),
        sink,
        warnings,
    )?;
    Ok(())
}
