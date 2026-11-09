use super::errors::ledger_error;
use std::collections::BTreeSet;
use vpt_application::ports::{
    EventSink, HistoryReceipt, HistoryWrite, SinkError, SinkOutcome, SinkWarning,
};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn finish<T>(
    ledger: &impl vpt_application::ports::RunHistory,
    run: vpt_domain::history::RunId,
    result: Result<T, Box<ErrorDocument>>,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<T, Box<ErrorDocument>> {
    use vpt_application::ports::{Clock, WorkDelta};
    use vpt_domain::run::HistoryFact;
    if result.as_ref().is_err_and(|error| {
        error.kind == ErrorKind::Ledger || error.rule.as_deref() == Some("unsafe_state")
    }) {
        return result;
    }
    let finish = commit(
        ledger.commit_step(
            run,
            &WorkDelta::default(),
            &[(
                vpt_adapters::SystemClock.now(),
                HistoryFact::RunFinished(super::run_outcome::from_result(&result)),
            )],
        ),
        sink,
        warnings,
    );
    match result {
        Ok(value) => finish.map(|_| value),
        Err(mut error) => {
            if let Err(finish) = finish {
                super::publication::merge_completed(&mut error, finish.completed);
            }
            Err(error)
        }
    }
}

pub(crate) fn commit<E: EventSink>(
    write: HistoryWrite<HistoryReceipt>,
    sink: &mut E,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<HistoryReceipt, Box<ErrorDocument>> {
    let receipt = match write {
        HistoryWrite::Committed(receipt) => receipt,
        HistoryWrite::CommittedWithFailure { receipt, error } => {
            return Err(Box::new(
                ledger_error(error).completed(receipt.completed_ids()),
            ));
        }
        HistoryWrite::Unconfirmed(error) => return Err(Box::new(ledger_error(error))),
    };
    match sink.after_commit(&receipt) {
        SinkOutcome::Continue { warnings: observed } => warnings.extend(observed),
        SinkOutcome::Stop(stop) => {
            warnings.extend(stop.warnings);
            let error = match stop.reason {
                SinkError::RequiredHistory(error) => ledger_error(error),
                SinkError::UnsafeState => {
                    ErrorDocument::new(ErrorKind::Refused, "state directory is unsafe")
                        .rule("unsafe_state")
                }
            };
            return Err(Box::new(error.completed(receipt.completed_ids())));
        }
    }
    Ok(receipt)
}

pub(crate) fn recover(
    ledger: &impl vpt_application::ports::RunHistory,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<(), Box<ErrorDocument>> {
    let view = ledger
        .recovery_view()
        .map_err(|error| Box::new(ledger_error(error)))?;
    match sink.recover(&view) {
        SinkOutcome::Continue { warnings: observed } => {
            warnings.extend(observed);
            Ok(())
        }
        SinkOutcome::Stop(stop) => {
            warnings.extend(stop.warnings);
            Err(Box::new(match stop.reason {
                SinkError::RequiredHistory(error) => ledger_error(error),
                SinkError::UnsafeState => {
                    ErrorDocument::new(ErrorKind::Refused, "state directory is unsafe")
                        .rule("unsafe_state")
                }
            }))
        }
    }
}
