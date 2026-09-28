use crate::RootDir;
use std::collections::BTreeSet;
use vpt_application::ports::{
    EventSink, HistoryReceipt, LedgerError, RunHistory, SinkError, SinkOutcome, SinkStop,
    SinkWarning,
};
use vpt_domain::run::RecoveryView;
use vpt_domain::time::UtcInstant;

mod export;
mod hook;
mod recovery;

pub struct LoggedEvents<'a, H: RunHistory, N: FnMut() -> UtcInstant> {
    state: &'a RootDir,
    history: &'a H,
    report_command: &'a [String],
    alert_command: &'a [String],
    clock: N,
}

impl<'a, H: RunHistory, N: FnMut() -> UtcInstant> LoggedEvents<'a, H, N> {
    pub fn new(
        state: &'a RootDir,
        history: &'a H,
        report_command: &'a [String],
        alert_command: &'a [String],
        clock: N,
    ) -> Self {
        Self {
            state,
            history,
            report_command,
            alert_command,
            clock,
        }
    }
}

impl<H: RunHistory, N: FnMut() -> UtcInstant> EventSink for LoggedEvents<'_, H, N> {
    fn after_commit(&mut self, committed: &HistoryReceipt) -> SinkOutcome {
        let mut warnings = BTreeSet::new();
        if let Err(stop) = self.export_pending(Some(committed), &mut warnings) {
            return SinkOutcome::Stop(stop);
        }
        for record in &committed.records {
            if let Err(stop) = self.invoke_hook(record, committed, &mut warnings) {
                return SinkOutcome::Stop(stop);
            }
        }
        SinkOutcome::Continue { warnings }
    }

    fn recover(&mut self, view: &RecoveryView) -> SinkOutcome {
        self.recover_history(view)
    }
}

fn stop(
    reason: SinkError,
    committed: Option<&HistoryReceipt>,
    warnings: &BTreeSet<SinkWarning>,
) -> Box<SinkStop> {
    Box::new(SinkStop {
        reason,
        committed: committed.cloned(),
        warnings: warnings.clone(),
    })
}

fn required(
    error: LedgerError,
    committed: Option<&HistoryReceipt>,
    warnings: &BTreeSet<SinkWarning>,
) -> Box<SinkStop> {
    let reason = if matches!(&error, LedgerError::PathEscape(_)) {
        SinkError::UnsafeState
    } else {
        SinkError::RequiredHistory(error)
    };
    stop(reason, committed, warnings)
}

#[cfg(test)]
mod tests;
