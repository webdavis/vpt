use super::{ReconcileProgress, Retention, RetentionReport, history, reconcile_intents};
use crate::ports::{
    Clock, EventSink, RetentionHistory, RetentionJournal, RunHistory, Stores, Trash,
};
use vpt_domain::history::RunId;

impl<L, S, T, C, E> Retention<'_, L, S, T, C, E>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores + crate::ports::RetentionFiles,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    pub(super) fn resume_and_sweep(
        &mut self,
        run: RunId,
        report: &mut RetentionReport,
        include_new: bool,
    ) -> Result<(), Box<history::Failure>> {
        match reconcile_intents(
            run,
            self.ledger,
            self.stores,
            self.trash,
            self.clock,
            self.paths,
            self.sink,
        ) {
            Ok(progress) => merge(report, progress),
            Err(failure) => {
                merge(report, failure.progress);
                return Err(Box::new(history::Failure {
                    error: failure.error,
                    committed: failure.committed,
                }));
            }
        }
        if include_new {
            self.sweep(Some(run), report)
        } else {
            Ok(())
        }
    }
}

fn merge(report: &mut RetentionReport, progress: ReconcileProgress) {
    report.moved.extend(progress.moved);
    for id in progress.completed {
        report.completed(&id);
    }
    report.warnings.extend(progress.warnings);
    report.diagnostics.extend(progress.diagnostics);
}
