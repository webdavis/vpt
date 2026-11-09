use super::candidate::recorder_failure;
use super::deferral::Deferral;
use super::failure::archive_failure;
use super::{Ingest, IngestError, IngestFailure, IngestReport, Mode};
use crate::ports::{
    Archive, Candidate, Clock, RecorderStore, RecordingLedger, RunHistory, SeenRow, Trash,
};
use vpt_domain::container::inspect;
use vpt_domain::history::RunId;
use vpt_domain::sweep::DeferralReason;
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
    pub(super) fn open_and_publish(
        &mut self,
        run: RunId,
        candidate: &Candidate,
        seen: Option<SeenRow>,
        now: UtcInstant,
        mode: &Mode,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let eligible =
            super::gates::source(self.recorder, candidate, seen.as_ref(), now, self.settings)
                .map_err(|failure| Self::error(report, failure, None))?;
        let eligible = match eligible {
            Ok(eligible) => eligible,
            Err(reason) => {
                return self.defer(
                    run,
                    Deferral {
                        candidate,
                        seen,
                        reason,
                    },
                    now,
                    mode,
                    report,
                );
            }
        };
        let source = eligible.handle;
        let before = eligible.metadata;
        let staged = self
            .archive
            .stage(|directory, identity, name| {
                self.recorder.clone_into(&source, directory, identity, name)
            })
            .map_err(|failure| {
                let cause = archive_failure(failure.cause);
                match failure.owned_staging {
                    Some(staged) => Self::owned_error(report, cause, staged),
                    None => Self::error(report, cause, None),
                }
            })?;
        let after = self.recorder.metadata(&source).map_err(|error| {
            Self::owned_error(report, recorder_failure(error), staged.path.clone())
        })?;
        if after != before {
            return self.defer_staged(
                run,
                Deferral {
                    candidate,
                    seen,
                    reason: DeferralReason::ChangedDuringRead,
                },
                &staged.path,
                now,
                mode,
                report,
            );
        }
        let archived = self.archive.open(&staged.path).map_err(|error| {
            Self::owned_error(report, archive_failure(error), staged.path.clone())
        })?;
        let size = self.archive.size(&archived).map_err(|error| {
            Self::owned_error(report, archive_failure(error), staged.path.clone())
        })?;
        let container = inspect(size, |offset, buffer| {
            self.archive.read_at(&archived, offset, buffer)
        });
        let consistent = if size == staged.size {
            self.archive
                .digest_handle(&archived)
                .map(|digest| digest == staged.digest)
        } else {
            Ok(false)
        };
        if consistent != Ok(true) {
            report.log.push(
                "staged cleanup pending: archive could not be verified against staged bytes".into(),
            );
            let failure = consistent.err().map_or_else(
                || IngestFailure::Sync("archive changed since staging".into()),
                archive_failure,
            );
            return Err(Self::error(report, failure, None));
        }
        let Ok(container) = container else {
            return self.defer_staged(
                run,
                Deferral {
                    candidate,
                    seen,
                    reason: DeferralReason::InvalidContainer,
                },
                &staged.path,
                now,
                mode,
                report,
            );
        };
        self.publish_staged(
            run, candidate, seen, now, staged, &archived, container, report,
        )
    }
}
