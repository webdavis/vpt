use super::{Ingest, IngestError, IngestFailure, IngestReport, failure::archive_failure};
use crate::ports::{
    Archive, Clock, EventSink, LedgerCommit, RecorderStore, RecordingLedger, RecordingRecord,
    RunHistory, StageStates, TitleOrigin, Trash, WorkDelta,
};
use vpt_domain::container::inspect;
use vpt_domain::history::RunId;
use vpt_domain::identity::RecordingId;
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
    pub(super) fn recover_archives(
        &mut self,
        run: RunId,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let paths = self
            .archive
            .archived()
            .map_err(|error| Self::error(report, archive_failure(error), None))?;
        for path in paths {
            let Some(id) = path
                .file_stem()
                .and_then(|name| name.to_str())
                .and_then(|name| RecordingId::parse(name).ok())
            else {
                continue;
            };
            if self
                .ledger
                .by_id(&id)
                .map_err(|error| Self::error(report, IngestFailure::Ledger(error), None))?
                .is_some()
            {
                continue;
            }
            let handle = self
                .archive
                .open(&path)
                .map_err(|error| Self::error(report, archive_failure(error), None))?;
            let size = self
                .archive
                .size(&handle)
                .map_err(|error| Self::error(report, archive_failure(error), None))?;
            if size > self.settings.max_audio_bytes {
                continue;
            }
            let Ok(container) = inspect(size, |offset, bytes| {
                self.archive.read_at(&handle, offset, bytes)
            }) else {
                continue;
            };
            let digest = self
                .archive
                .digest_handle(&handle)
                .map_err(|error| Self::error(report, archive_failure(error), None))?;
            if digest.hash12() != id.hash12() {
                continue;
            }
            let Some(offset) = id.archived_offset(container.creation_time) else {
                continue;
            };
            if self
                .ledger
                .by_digest(&digest)
                .map_err(|error| Self::error(report, IngestFailure::Ledger(error), None))?
                .is_some()
            {
                continue;
            }
            self.archive
                .sync_existing(&path, &digest)
                .map_err(|error| Self::error(report, archive_failure(error), None))?;
            let now = self.clock.now();
            let record = RecordingRecord {
                id: id.clone(),
                source_path: None,
                digest,
                captured_at: container.creation_time,
                captured_offset: offset,
                duration_secs: container.duration_secs,
                title: None,
                title_source: TitleOrigin::Unavailable,
                ingested_at: now,
                audio_path: path,
                stages: StageStates::fresh(),
                audio_trashed_at: None,
            };
            let receipt = Self::required(
                self.ledger.commit_step(
                    run,
                    &WorkDelta {
                        commit: LedgerCommit {
                            recordings: vec![record],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    &[(
                        now,
                        HistoryFact::WorkStep {
                            recording: Some(id.clone()),
                            outcome: WorkOutcome::Completed,
                        },
                    )],
                ),
                report,
            )?;
            report.recovered.push(id);
            self.export(&receipt, report)?;
        }
        Ok(())
    }
}
