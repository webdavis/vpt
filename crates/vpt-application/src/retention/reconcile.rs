mod effects;
mod intent;
use super::{ReconcileFailure, ReconcileProgress, RetentionError};
use crate::StorePaths;
use crate::ports::{
    Clock, EventSink, RetentionHistory, RetentionJournal, RunHistory, Stores, Trash,
};
use vpt_domain::history::RunId;

pub fn reconcile_intents<L, S, T, C, E>(
    run: RunId,
    ledger: &L,
    stores: &S,
    trash: &T,
    clock: &C,
    paths: &StorePaths,
    sink: &mut E,
) -> Result<ReconcileProgress, Box<ReconcileFailure>>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    Reconciler {
        run,
        ledger,
        stores,
        trash,
        clock,
        paths,
        sink,
        progress: ReconcileProgress::default(),
    }
    .run()
}

struct Reconciler<'a, L, S, T, C, E> {
    run: RunId,
    ledger: &'a L,
    stores: &'a S,
    trash: &'a T,
    clock: &'a C,
    paths: &'a StorePaths,
    sink: &'a mut E,
    progress: ReconcileProgress,
}

impl<L, S, T, C, E> Reconciler<'_, L, S, T, C, E>
where
    L: RetentionJournal + RetentionHistory + RunHistory + crate::ports::RetentionArtifacts,
    S: Stores,
    T: Trash,
    C: Clock,
    E: EventSink,
{
    fn run(mut self) -> Result<ReconcileProgress, Box<ReconcileFailure>> {
        let intents = self
            .ledger
            .pending_intents()
            .map_err(|error| self.failed(RetentionError::Ledger(error), None))?;
        if intents.is_empty() {
            return Ok(self.progress);
        }
        let view = self
            .ledger
            .recovery_view()
            .map_err(|error| self.failed(RetentionError::RequiredHistory(error), None))?;
        let owned = self
            .ledger
            .retention_artifacts()
            .map_err(|error| self.failed(RetentionError::Ledger(error), None))?;
        for intent in intents {
            let artifact = owned
                .iter()
                .find(|row| row.matches_intent(&intent))
                .ok_or_else(|| {
                    self.failed(RetentionError::TargetModified(intent.path.clone()), None)
                })?;
            self.one(&intent, artifact, &view.records)?;
        }
        Ok(self.progress)
    }
}
