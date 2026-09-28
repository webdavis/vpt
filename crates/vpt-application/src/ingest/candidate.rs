use super::{Ingest, IngestError, IngestFailure, IngestReport};
use crate::ports::{
    Archive, Candidate, Clock, RecorderError, RecorderStore, RecordingLedger, RunHistory, SeenRow,
    Trash, WorkDelta,
};
use vpt_domain::history::RunId;
use vpt_domain::run::{HistoryFact, WorkOutcome};
use vpt_domain::sweep::{CandidateFacts, PreOpen, SeenFacts, pre_open};
use vpt_domain::time::UtcInstant;

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: crate::ports::EventSink + ?Sized,
{
    pub(super) fn process_candidate(
        &mut self,
        run: RunId,
        candidate: &Candidate,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let seen = self
            .ledger
            .seen(&candidate.path)
            .map_err(|error| Self::error(report, IngestFailure::Ledger(error), None))?;
        let facts = CandidateFacts {
            size: candidate.size,
            mtime: candidate.mtime,
            flags: candidate.flags,
        };
        let previous = seen.as_ref().map(|row| SeenFacts {
            size: row.size,
            mtime: row.mtime,
            ingested: row.recording.is_some() && row.deferral_reason.is_none(),
            deferred_size: row.deferred_size,
        });
        match pre_open(&facts, previous.as_ref()) {
            PreOpen::Dataless => Err(Self::error(
                report,
                IngestFailure::DatalessUnavailable(candidate.path.clone()),
                None,
            )),
            PreOpen::Unchanged => match seen {
                Some(row) => self.refresh_liveness(run, row, now, report),
                None => self.open_and_publish(run, candidate, None, now, report),
            },
            PreOpen::Open => self.open_and_publish(run, candidate, seen, now, report),
        }
    }

    fn refresh_liveness(
        &mut self,
        run: RunId,
        mut row: SeenRow,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        row.last_seen = now;
        row.source_gone_at = None;
        let recorded = row.recording.clone();
        let delta = WorkDelta {
            commit: crate::ports::LedgerCommit {
                seen: vec![row],
                ..Default::default()
            },
            ..Default::default()
        };
        let receipt = Self::required(
            self.ledger.commit_step(
                run,
                &delta,
                &[(
                    now,
                    HistoryFact::WorkStep {
                        recording: recorded.clone(),
                        outcome: WorkOutcome::Completed,
                    },
                )],
            ),
            report,
        )?;
        self.export(&receipt, report)?;
        report.skipped += 1;
        if let Some(id) = recorded {
            report.already_ingested.push(id);
        }
        Ok(())
    }
}

pub(super) fn recorder_failure(error: RecorderError) -> IngestFailure {
    match error {
        RecorderError::Escape(path) | RecorderError::NotRegular(path) => {
            IngestFailure::PathEscape(path)
        }
        RecorderError::Unreadable(detail) => IngestFailure::StoreUnreadable(detail),
        RecorderError::NotFound(path) => IngestFailure::OnceMissing(path),
        RecorderError::Io(detail) => IngestFailure::Recorder(detail),
    }
}

pub(super) fn fresh_seen(candidate: &Candidate, now: UtcInstant) -> SeenRow {
    SeenRow {
        path: candidate.path.clone(),
        file_name: candidate.file_name.clone(),
        size: candidate.size,
        mtime: candidate.mtime,
        flags: candidate.flags,
        first_seen: now,
        last_seen: now,
        deferral_count: 0,
        deferral_reason: None,
        deferred_size: None,
        source_gone_at: None,
        recording: None,
    }
}
