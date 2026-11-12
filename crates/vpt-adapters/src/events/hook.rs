use super::{LoggedEvents, required};
use crate::{OUTPUT_LIMIT, SpawnError, Status, spawn};
use std::collections::BTreeSet;
use std::time::Duration;
use vpt_application::ports::{
    HistoryReceipt, HistoryWrite, RunHistory, SinkStop, SinkWarning, WorkDelta,
};
use vpt_domain::history::Event;
use vpt_domain::run::{HistoryFact, HistoryRecord, HookFailureClass};
use vpt_domain::time::UtcInstant;
use vpt_protocol::encode_hook;

impl<H: RunHistory, N: FnMut() -> UtcInstant> LoggedEvents<'_, H, N> {
    pub(super) fn invoke_hook(
        &mut self,
        record: &HistoryRecord,
        committed: &HistoryReceipt,
        warnings: &mut BTreeSet<SinkWarning>,
    ) -> Result<(), Box<SinkStop>> {
        let HistoryFact::Event(event) = &record.fact else {
            return Ok(());
        };
        let command = self.command(event);
        if command.is_empty() {
            return Ok(());
        }
        let failure = match encode_hook(event) {
            Ok(document) => classify(spawn::run(
                command,
                &document,
                Duration::from_secs(5),
                OUTPUT_LIMIT,
            )),
            Err(_) => Some(HookFailureClass::Encoding),
        };
        let Some(failure) = failure else {
            return Ok(());
        };
        warnings.insert(SinkWarning::HookFailed);
        let at = (self.clock)();
        let fact = HistoryFact::HookFailed {
            event: record.id,
            failure,
        };
        let receipt =
            match self
                .history
                .commit_step(committed.run, &WorkDelta::default(), &[(at, fact)])
            {
                HistoryWrite::Committed(receipt) => receipt,
                HistoryWrite::CommittedWithFailure { receipt, error } => {
                    return Err(required(error, Some(&receipt), warnings));
                }
                HistoryWrite::Unconfirmed(error) => {
                    return Err(required(error, Some(committed), warnings));
                }
            };
        if !warnings.contains(&SinkWarning::ExportUnavailable)
            && !warnings.contains(&SinkWarning::ExportProgressUnconfirmed)
        {
            self.export_pending(Some(&receipt), warnings)?;
        }
        Ok(())
    }

    fn command(&self, event: &Event) -> &[String] {
        if event.kind.is_report() {
            self.report_command
        } else {
            self.alert_command
        }
    }
}

fn classify(result: Result<spawn::Outcome, SpawnError>) -> Option<HookFailureClass> {
    match result {
        Ok(outcome) => match outcome.status {
            Status::Exited(0) => None,
            Status::Exited(code) => Some(HookFailureClass::Exited(code)),
            Status::Signaled(signal) => Some(HookFailureClass::Signaled(signal)),
            Status::DeadlineExceeded => Some(HookFailureClass::DeadlineExceeded),
            Status::Interrupted => Some(HookFailureClass::Interrupted),
        },
        Err(SpawnError::NotFound(_)) => Some(HookFailureClass::Spawn),
        Err(SpawnError::Io(_)) => Some(HookFailureClass::Io),
    }
}
