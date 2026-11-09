use super::{Ingest, IngestError, IngestFailure, IngestReport, Mode, candidate::recorder_failure};
use crate::ports::{
    Archive, Candidate, Clock, EventSink, LedgerCommit, RecorderStore, RecordingLedger, RunHistory,
    Trash, WorkDelta,
};
use vpt_domain::history::RunId;
use vpt_domain::run::{HistoryFact, WorkOutcome};

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    pub(super) fn select(
        &self,
        mode: &Mode,
        report: &IngestReport,
    ) -> Result<Vec<Candidate>, Box<IngestError>> {
        let candidates = match &mode.once {
            Some(path) => self
                .recorder
                .candidate(path)
                .map(|candidate| vec![candidate]),
            None => self.recorder.candidates(),
        }
        .map_err(|error| Self::error(report, recorder_failure(error), None))?;
        if mode.once.is_none() && candidates.is_empty() {
            let seen = self
                .ledger
                .seen_all()
                .map_err(|error| Self::error(report, IngestFailure::Ledger(error), None))?;
            if seen
                .iter()
                .any(|row| row.path.parent() == Some(self.settings.recordings_dir.as_path()))
            {
                return Err(Self::error(report, IngestFailure::EmptyStore, None));
            }
        }
        Ok(candidates)
    }

    pub(super) fn mark_vanished(
        &mut self,
        run: RunId,
        candidates: &[Candidate],
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let mut gone = self
            .ledger
            .seen_all()
            .map_err(|error| Self::error(report, IngestFailure::Ledger(error), None))?;
        gone.retain(|row| {
            row.recording.is_some()
                && row.source_gone_at.is_none()
                && row.path.parent() == Some(self.settings.recordings_dir.as_path())
                && !candidates
                    .iter()
                    .any(|candidate| candidate.path == row.path)
        });
        if gone.is_empty() {
            return Ok(());
        }
        let now = self.clock.now();
        for row in &mut gone {
            row.source_gone_at = Some(now);
        }
        let facts = gone
            .iter()
            .map(|row| {
                (
                    now,
                    HistoryFact::WorkStep {
                        recording: row.recording.clone(),
                        outcome: WorkOutcome::Completed,
                    },
                )
            })
            .collect::<Vec<_>>();
        let receipt = Self::required(
            self.ledger.commit_step(
                run,
                &WorkDelta {
                    commit: LedgerCommit {
                        seen: gone,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                &facts,
            ),
            report,
        )?;
        self.export(&receipt, report)
    }
}
