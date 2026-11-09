use super::{Ingest, IngestError, IngestFailure, IngestReport, failure::archive_failure};
use crate::ports::{
    Archive, Clock, EventSink, RecorderStore, RecordingLedger, RunHistory, Staged, Trash,
};
use vpt_domain::container::inspect;

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    pub(super) fn validate_existing(
        &self,
        path: &std::path::Path,
        staged: &Staged,
        report: &IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let failure =
            |error| Self::owned_error(report, archive_failure(error), staged.path.clone());
        let digest = self.archive.digest(path).map_err(failure)?;
        if digest != staged.digest {
            return Err(Self::owned_error(
                report,
                IngestFailure::ArchiveCollision {
                    target: path.into(),
                    staged: staged.digest,
                    existing: digest,
                },
                staged.path.clone(),
            ));
        }
        let handle = self.archive.open(path).map_err(failure)?;
        let size = self.archive.size(&handle).map_err(failure)?;
        inspect(size, |offset, bytes| {
            self.archive.read_at(&handle, offset, bytes)
        })
        .map_err(|_| {
            Self::owned_error(
                report,
                IngestFailure::Archive("existing container is invalid".into()),
                staged.path.clone(),
            )
        })?;
        let digest = self.archive.digest_handle(&handle).map_err(failure)?;
        if digest != staged.digest {
            return Err(Self::owned_error(
                report,
                IngestFailure::ArchiveCollision {
                    target: path.into(),
                    staged: staged.digest,
                    existing: digest,
                },
                staged.path.clone(),
            ));
        }
        self.archive
            .sync_existing(path, &staged.digest)
            .map_err(failure)
    }
}
