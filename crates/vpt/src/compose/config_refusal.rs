use super::{errors::root_error, history::commit};
use std::collections::BTreeSet;
use vpt_adapters::{RootError, SystemClock};
use vpt_application::ports::{Clock, EventSink, RunHistory, SinkWarning, WorkDelta};
use vpt_domain::{
    history::{Event, EventCounts, EventDetail, EventKind, EventState, RunId, SafeFailureClass},
    run::{HistoryFact, RunKind, RunOutcome},
};
use vpt_protocol::error::ErrorDocument;

pub(crate) fn start_run(
    ledger: &impl RunHistory,
    sink: &mut impl EventSink,
    kind: RunKind,
    cause: RootError,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Box<ErrorDocument> {
    match commit(ledger.begin_run(kind, SystemClock.now()), sink, warnings) {
        Ok(receipt) => finish_run(ledger, sink, receipt.run, cause, warnings),
        Err(error) => error,
    }
}

pub(crate) fn in_run(
    ledger: &impl RunHistory,
    sink: &mut impl EventSink,
    run: RunId,
    cause: RootError,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Box<ErrorDocument> {
    save(ledger, sink, run, cause, warnings, false)
}

pub(crate) fn finish_run(
    ledger: &impl RunHistory,
    sink: &mut impl EventSink,
    run: RunId,
    cause: RootError,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Box<ErrorDocument> {
    save(ledger, sink, run, cause, warnings, true)
}

fn save(
    ledger: &impl RunHistory,
    sink: &mut impl EventSink,
    run: RunId,
    cause: RootError,
    warnings: &mut BTreeSet<SinkWarning>,
    finish: bool,
) -> Box<ErrorDocument> {
    let at = SystemClock.now();
    let mut facts = vec![(
        at,
        HistoryFact::Event(Event {
            kind: EventKind::ConfigRefused,
            state: EventState::Failed,
            occasion: None,
            recording: None,
            detail: EventDetail::Failure(SafeFailureClass::ConfigurationRefused),
            counts: EventCounts::default(),
            paths: vec![],
            occurred_at: at,
        }),
    )];
    if finish {
        facts.push((at, HistoryFact::RunFinished(RunOutcome::Failed)));
    }
    commit(
        ledger.commit_step(run, &WorkDelta::default(), &facts),
        sink,
        warnings,
    )
    .err()
    .unwrap_or_else(|| Box::new(root_error(&cause)))
}
