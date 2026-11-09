mod candidate;
mod cleanup;
mod commit;
mod deferral;
mod existing;
mod failure;
mod gates;
mod history;
mod liveness;
mod observe;
mod publish;
mod recovery;
mod report;
mod staging;
pub use report::*;

use crate::SourceSettings;
use crate::ports::{
    Archive, Clock, EventSink, RecorderStore, RecordingLedger, RunHistory, Trash, WorkDelta,
};
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};

pub struct Ingest<'a, R, A, L, C, T: ?Sized, S: ?Sized> {
    pub recorder: &'a R,
    pub archive: &'a A,
    pub ledger: &'a L,
    pub clock: &'a C,
    pub trash: &'a T,
    pub sink: &'a mut S,
    pub settings: &'a SourceSettings,
}

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    pub fn run(&mut self, mode: &Mode) -> Result<IngestReport, Box<IngestError>> {
        let mut report = IngestReport::default();
        if mode.dry_run {
            return self.observe(mode);
        }
        let started_at = self.clock.now();
        let started = Self::required(self.ledger.begin_run(RunKind::Ingest, started_at), &report)?;
        self.export(&started, &mut report)?;
        if let Err(error) = self.recover_archives(started.run, &mut report) {
            return Err(self.finish_failure(started.run, self.clock.now(), error, &mut report));
        }
        if let Err(error) = self.recorder.refresh_titles() {
            let failure = Self::error(&report, candidate::recorder_failure(error), None);
            return Err(self.finish_failure(started.run, self.clock.now(), failure, &mut report));
        }
        let candidates = match self.select(mode, &report) {
            Ok(candidates) => candidates,
            Err(error) => {
                return Err(self.finish_failure(started.run, self.clock.now(), error, &mut report));
            }
        };
        if mode.once.is_none() {
            self.mark_vanished(started.run, &candidates, &mut report)?;
        }
        for candidate in &candidates {
            let now = self.clock.now();
            if let Err(error) =
                self.process_candidate(started.run, candidate, now, mode, &mut report)
            {
                return Err(self.finish_failure(started.run, now, error, &mut report));
            }
        }
        let finished = Self::required(
            self.ledger.commit_step(
                started.run,
                &WorkDelta::default(),
                &[(
                    self.clock.now(),
                    HistoryFact::RunFinished(RunOutcome::Completed),
                )],
            ),
            &report,
        )?;
        self.export(&finished, &mut report)?;
        Ok(report)
    }
}
