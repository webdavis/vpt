mod event;
mod expire;
mod history;
mod reconcile;
mod report;
mod resume;
mod sweep;
use crate::ports::{
    Clock, EventSink, RetentionHistory, RetentionJournal, RunHistory, Stores, Trash,
};
use crate::{RetentionSettings, StorePaths};
pub use reconcile::reconcile_intents;
pub use report::{
    Kept, KeptReason, Moved, ReconcileFailure, ReconcileProgress, RetentionCommit, RetentionError,
    RetentionFailure, RetentionReport,
};
use vpt_domain::history::RunId;
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};

pub struct Retention<'a, L, S, T, C, E> {
    pub ledger: &'a L,
    pub stores: &'a S,
    pub trash: &'a T,
    pub clock: &'a C,
    pub sink: &'a mut E,
    pub settings: &'a RetentionSettings,
    pub paths: &'a StorePaths,
}

impl<L, S, T, C, E> Retention<'_, L, S, T, C, E>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores + crate::ports::RetentionFiles,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    pub fn run(
        &mut self,
        dry_run: bool,
        initial: ReconcileProgress,
    ) -> Result<RetentionReport, Box<RetentionFailure>> {
        self.execute(dry_run, initial, true)
    }

    pub fn recover(&mut self) -> Result<RetentionReport, Box<RetentionFailure>> {
        self.execute(false, ReconcileProgress::default(), false)
    }

    /// The caller commits the combined retention event and finishes this existing history run.
    pub fn run_in(&mut self, run: RunId) -> Result<RetentionReport, Box<RetentionFailure>> {
        let mut report = RetentionReport::default();
        if !self.settings.enabled {
            return Err(report.failure(RetentionError::Disabled, None));
        }
        self.resume_and_sweep(run, &mut report, true)
            .map_err(|failure| report.failure(failure.error, failure.committed))?;
        Ok(report)
    }

    fn execute(
        &mut self,
        dry_run: bool,
        initial: ReconcileProgress,
        include_new: bool,
    ) -> Result<RetentionReport, Box<RetentionFailure>> {
        let mut report = RetentionReport::from(initial);
        if include_new && !self.settings.enabled {
            return Err(report.failure(RetentionError::Disabled, None));
        }
        if dry_run {
            self.sweep(None, &mut report)
                .map_err(|failure| report.failure(failure.error, failure.committed))?;
            return Ok(report);
        }
        let receipt = history::required(self.ledger.begin_run(RunKind::Retain, self.clock.now()))
            .map_err(|failure| report.failure(failure.error, failure.committed))?;
        history::export(self.sink, &receipt, &mut report.warnings)
            .map_err(|failure| report.failure(failure.error, failure.committed))?;
        let failed = match self.resume_and_sweep(receipt.run, &mut report, include_new) {
            Err(failure)
                if matches!(
                    failure.error,
                    RetentionError::RequiredHistory(_) | RetentionError::SinkStopped(_)
                ) =>
            {
                return Err(report.failure(failure.error, failure.committed));
            }
            result => result.err(),
        };
        self.finish(receipt.run, failed.is_some(), &mut report)
            .map_err(|failure| report.failure(failure.error, failure.committed))?;
        match failed {
            Some(failure) => Err(report.failure(failure.error, failure.committed)),
            None => Ok(report),
        }
    }

    fn finish(
        &mut self,
        run: RunId,
        failed: bool,
        report: &mut RetentionReport,
    ) -> Result<(), Box<history::Failure>> {
        let at = self.clock.now();
        let mut facts = Vec::new();
        if let Some(event) = report.event(at, failed).map_err(history::required_error)? {
            facts.push((at, HistoryFact::Event(event)));
        }
        facts.push((
            at,
            HistoryFact::RunFinished(if failed {
                RunOutcome::Failed
            } else {
                RunOutcome::Completed
            }),
        ));
        history::append(self.ledger, self.sink, run, &facts, &mut report.warnings)?;
        Ok(())
    }
}
