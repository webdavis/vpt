use super::{
    Deferred, Ingest, IngestError, IngestFailure, IngestReport, Mode, WouldIngest,
    candidate::recorder_failure, gates,
};
use crate::ports::{
    Archive, Clock, EventSink, RecorderStore, RecordingLedger, RunHistory, TitleOrigin, Trash,
};
use vpt_domain::sweep::{DeferralReason, PreOpen};

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    pub(super) fn observe(&self, mode: &Mode) -> Result<IngestReport, Box<IngestError>> {
        let mut report = IngestReport::default();
        let candidates = self.select(mode, &report)?;
        for candidate in candidates {
            let mut known = None;
            let seen = self
                .ledger
                .seen(&candidate.path)
                .map_err(|error| Self::error(&report, IngestFailure::Ledger(error), None))?;
            let reason = match gates::selection(&candidate, seen.as_ref()) {
                PreOpen::Dataless => Some(DeferralReason::Dataless),
                PreOpen::Unchanged => {
                    report.skipped += 1;
                    if let Some(id) = seen.and_then(|row| row.recording) {
                        report.already_ingested.push(id);
                    }
                    continue;
                }
                PreOpen::Open => match gates::source(
                    self.recorder,
                    &candidate,
                    seen.as_ref(),
                    self.clock.now(),
                    self.settings,
                    mode,
                )
                .map_err(|failure| Self::error(&report, failure, None))?
                {
                    Err(reason) => Some(reason),
                    Ok(source) => {
                        let digest = self
                            .recorder
                            .digest(&source.handle, source.metadata.size)
                            .map_err(|error| Self::error(&report, recorder_failure(error), None))?;
                        let after = self
                            .recorder
                            .metadata(&source.handle)
                            .map_err(|error| Self::error(&report, recorder_failure(error), None))?;
                        if after != source.metadata {
                            Some(DeferralReason::ChangedDuringRead)
                        } else {
                            known = self.ledger.by_digest(&digest).map_err(|error| {
                                Self::error(&report, IngestFailure::Ledger(error), None)
                            })?;
                            let captured = source.container.creation_time;
                            (known.is_none()
                                && vpt_domain::identity::RecordingId::derive(
                                    captured,
                                    self.clock.offset_at(captured),
                                    &digest,
                                )
                                .is_err())
                            .then_some(DeferralReason::InvalidContainer)
                        }
                    }
                },
            };
            if let Some(reason) = reason {
                report.deferred.push(Deferred {
                    path: candidate.path,
                    reason,
                });
            } else {
                report.would_ingest.push(WouldIngest {
                    path: candidate.path,
                    title: known.as_ref().and_then(|record| record.title.clone()),
                    title_source: known
                        .map_or(TitleOrigin::Unavailable, |record| record.title_source),
                });
            }
        }
        Ok(report)
    }
}
