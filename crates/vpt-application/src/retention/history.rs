use super::{RetentionCommit, RetentionError};
use crate::ports::{
    EventSink, HistoryReceipt, HistoryWrite, LedgerError, RunHistory, SinkOutcome, SinkWarning,
    WorkDelta,
};
use std::collections::BTreeSet;
use vpt_domain::history::RunId;
use vpt_domain::run::HistoryFact;
use vpt_domain::time::UtcInstant;

pub(super) struct Failure {
    pub error: RetentionError,
    pub committed: Option<RetentionCommit>,
}

pub(super) fn failure(error: RetentionError) -> Box<Failure> {
    Box::new(Failure {
        error,
        committed: None,
    })
}

pub(super) fn required_error(error: LedgerError) -> Box<Failure> {
    failure(RetentionError::RequiredHistory(error))
}

pub(super) fn required(
    result: HistoryWrite<HistoryReceipt>,
) -> Result<HistoryReceipt, Box<Failure>> {
    match result {
        HistoryWrite::Committed(receipt) => Ok(receipt),
        HistoryWrite::CommittedWithFailure { receipt, error } => Err(Box::new(Failure {
            error: RetentionError::RequiredHistory(error),
            committed: Some(RetentionCommit::Step(receipt)),
        })),
        HistoryWrite::Unconfirmed(error) => Err(Box::new(Failure {
            error: RetentionError::RequiredHistory(error),
            committed: None,
        })),
    }
}

pub(super) fn export<E: EventSink>(
    sink: &mut E,
    receipt: &HistoryReceipt,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<(), Box<Failure>> {
    match sink.after_commit(receipt) {
        SinkOutcome::Continue { warnings: observed } => {
            warnings.extend(observed);
            Ok(())
        }
        SinkOutcome::Stop(stopped) => {
            warnings.extend(stopped.warnings);
            Err(Box::new(Failure {
                error: RetentionError::SinkStopped(stopped.reason),
                committed: Some(RetentionCommit::Step(
                    stopped.committed.unwrap_or_else(|| receipt.clone()),
                )),
            }))
        }
    }
}

pub(super) fn append<L: RunHistory, E: EventSink>(
    ledger: &L,
    sink: &mut E,
    run: RunId,
    facts: &[(UtcInstant, HistoryFact)],
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<HistoryReceipt, Box<Failure>> {
    let receipt = required(ledger.commit_step(run, &WorkDelta::default(), facts))?;
    export(sink, &receipt, warnings)?;
    Ok(receipt)
}
