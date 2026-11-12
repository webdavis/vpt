use super::super::scan::Carrier;
use crate::compose::{errors::contained_error, history::commit, mentions::Context, publication};
use std::{collections::BTreeSet, path::PathBuf};
use vpt_adapters::{FilesystemStores, Roots, SqliteLedger, SystemClock, known_terms::Update};
use vpt_application::{Settings, ports::*};
use vpt_domain::{history::RunId, identity::RecordingId, run::*, tags::Rename, time::UtcInstant};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Work<'a, E: EventSink> {
    pub ledger: &'a SqliteLedger,
    pub sink: &'a mut E,
    pub warnings: &'a mut BTreeSet<SinkWarning>,
    pub settings: &'a Settings,
    pub roots: &'a Roots,
    pub context: &'a Context,
}

#[derive(Default)]
pub(super) struct Progress {
    pub renamed: Vec<(RecordingId, Vec<PathBuf>)>,
    pub failed: Vec<(RecordingId, Box<ErrorDocument>)>,
}

impl<E: EventSink> Work<'_, E> {
    pub(super) fn run(
        &mut self,
        rename: &Rename,
        known: &Update,
        carriers: Vec<Carrier>,
        completed: &mut Vec<String>,
    ) -> Result<Progress, Box<ErrorDocument>> {
        let started = self.ledger.begin_run(RunKind::Confirm, SystemClock.now());
        let run = commit(started, self.sink, self.warnings)?.run;
        let result = self.steps(run, rename, known, carriers, completed);
        let outcome = match &result {
            Ok(progress) if progress.failed.is_empty() => RunOutcome::Completed,
            _ => RunOutcome::Failed,
        };
        let finished = self.ledger.commit_step(
            run,
            &WorkDelta::default(),
            &[(SystemClock.now(), HistoryFact::RunFinished(outcome))],
        );
        commit(finished, self.sink, self.warnings)?;
        result
    }

    fn steps(
        &mut self,
        run: RunId,
        rename: &Rename,
        known: &Update,
        carriers: Vec<Carrier>,
        completed: &mut Vec<String>,
    ) -> Result<Progress, Box<ErrorDocument>> {
        let stores = if carriers.is_empty() {
            FilesystemStores::open_read_only(&[])
        } else {
            self.roots.create_leaves().map_err(|error| {
                crate::compose::config_refusal::in_run(
                    self.ledger,
                    self.sink,
                    run,
                    error,
                    self.warnings,
                )
            })?;
            let stores = &self.roots.stores;
            FilesystemStores::open(&[stores.transcripts.clone(), stores.analysis.clone()])
        }
        .map_err(|error| Box::new(contained_error(error)))?;
        self.record(run, rename, known, &stores)?;
        let mut progress = Progress::default();
        for carrier in carriers {
            let id = carrier.id.clone();
            match self.recording(run, carrier, &stores) {
                Ok(row) => progress.renamed.push(row),
                Err(error) if error.completed.contains(&id.to_string()) => {
                    progress.renamed.push((id.clone(), Vec::new()));
                }
                Err(error) => {
                    self.failed(run, &id)?;
                    progress.failed.push((id, error));
                    continue;
                }
            }
            completed.push(id.to_string());
        }
        Ok(progress)
    }

    fn failed(&mut self, run: RunId, id: &RecordingId) -> Result<(), Box<ErrorDocument>> {
        let fact = HistoryFact::WorkStep {
            recording: Some(id.clone()),
            outcome: WorkOutcome::Failed,
        };
        let step =
            self.ledger
                .commit_step(run, &WorkDelta::default(), &[(SystemClock.now(), fact)]);
        commit(step, self.sink, self.warnings).map(|_| ())
    }

    pub(super) fn commit(
        &mut self,
        run: RunId,
        delta: &WorkDelta,
        facts: &[(UtcInstant, HistoryFact)],
        stores: &FilesystemStores,
    ) -> Result<(), Box<ErrorDocument>> {
        commit(
            self.ledger.commit_step(run, delta, facts),
            self.sink,
            self.warnings,
        )?;
        publication::repair(self.ledger, stores, self.settings, self.sink, self.warnings)?;
        Ok(())
    }
}
