use super::candidate::fresh_seen;
use super::{Deferred, Ingest, IngestError, IngestReport, Mode};
use crate::ports::{
    Archive, Candidate, Clock, LedgerCommit, RecorderStore, RecordingLedger, RunHistory, SeenRow,
    Trash, TrashError, WorkDelta,
};
use std::path::Path;
use vpt_domain::history::{
    Event, EventCounts, EventDetail, EventKind, EventPath, EventPathKey, EventState, RunId,
};
use vpt_domain::run::{HistoryFact, WorkOutcome};
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::UtcInstant;

pub(super) struct Deferral<'a> {
    pub(super) candidate: &'a Candidate,
    pub(super) seen: Option<SeenRow>,
    pub(super) reason: DeferralReason,
}

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: crate::ports::EventSink + ?Sized,
{
    pub(super) fn defer(
        &mut self,
        run: RunId,
        deferral: Deferral<'_>,
        now: UtcInstant,
        mode: &Mode,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let Deferral {
            candidate,
            seen,
            reason,
        } = deferral;
        if mode.dry_run {
            report.deferred.push(Deferred {
                path: candidate.path.clone(),
                reason,
            });
            return Ok(());
        }
        let mut row = seen.unwrap_or_else(|| fresh_seen(candidate, now));
        let previous = row.deferral_count;
        row.size = candidate.size;
        row.mtime = candidate.mtime;
        row.flags = candidate.flags;
        row.last_seen = now;
        row.source_gone_at = None;
        row.deferral_count = previous.saturating_add(1);
        row.deferral_reason = Some(reason);
        row.deferred_size = Some(candidate.size);
        let threshold = self.settings.deferral_page_threshold;
        let mut facts = vec![(
            now,
            HistoryFact::WorkStep {
                recording: None,
                outcome: WorkOutcome::Deferred,
            },
        )];
        if previous < threshold && row.deferral_count >= threshold {
            facts.push((
                now,
                HistoryFact::Event(Event {
                    kind: EventKind::Deferred,
                    state: EventState::NeedsAttention,
                    recording: None,
                    detail: EventDetail::Deferred {
                        reason,
                        count: row.deferral_count,
                    },
                    counts: EventCounts::default(),
                    paths: vec![EventPath {
                        key: EventPathKey::Source,
                        path: candidate.path.clone(),
                    }],
                    occurred_at: now,
                }),
            ));
        }
        let delta = WorkDelta {
            commit: LedgerCommit {
                seen: vec![row],
                ..Default::default()
            },
            ..Default::default()
        };
        let receipt = Self::required(self.ledger.commit_step(run, &delta, &facts), report)?;
        self.export(&receipt, report)?;
        report.deferred.push(Deferred {
            path: candidate.path.clone(),
            reason,
        });
        Ok(())
    }

    pub(super) fn defer_staged(
        &mut self,
        run: RunId,
        deferral: Deferral<'_>,
        staged: &Path,
        now: UtcInstant,
        mode: &Mode,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        self.defer(run, deferral, now, mode, report)?;
        if mode.dry_run {
            return Ok(());
        }
        if let Some(TrashError::HelperVersion { found }) = self.discard_owned(staged, report) {
            return Err(Self::error(
                report,
                super::IngestFailure::HelperVersion { found },
                None,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
