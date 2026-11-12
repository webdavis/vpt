use super::failure::archive_failure;
use super::{Ingest, IngestError, IngestFailure, IngestReport, Mode, commit::Accepted};
use crate::ports::{
    Archive, ArchiveError, Candidate, Clock, EventSink, Published, RecorderStore, RecordingLedger,
    RecordingRecord, RunHistory, SeenRow, StageStates, Staged, TitleLookup, TitleOrigin, Trash,
};
use vpt_domain::container::Container;
use vpt_domain::history::RunId;
use vpt_domain::identity::RecordingId;
use vpt_domain::time::UtcInstant;

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    #[allow(
        clippy::too_many_arguments,
        reason = "staged publication carries the source and validated archive facts"
    )]
    pub(super) fn publish_staged(
        &mut self,
        run: RunId,
        candidate: &Candidate,
        seen: Option<SeenRow>,
        now: UtcInstant,
        staged: Staged,
        archived: &A::Handle,
        container: Container,
        mode: &Mode,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let known = self.ledger.by_digest(&staged.digest).map_err(|error| {
            Self::owned_error(report, IngestFailure::Ledger(error), staged.path.clone())
        })?;
        if !staged.copy_on_write {
            report.log.push("audio archive is not copy-on-write".into());
        }
        if let Some(record) = known {
            if record.audio_trashed_at.is_none() {
                self.validate_existing(&record.audio_path, &staged, report)?;
            }
            self.commit_record(
                run,
                candidate,
                seen,
                now,
                Accepted {
                    record,
                    existing: true,
                    intent: None,
                    source: mode.source(candidate).to_path_buf(),
                },
                report,
            )?;
            self.discard_owned(&staged.path, report);
            return Ok(());
        }
        let captured_at = mode.captured_at(&container, candidate);
        let offset = self.clock.offset_at(captured_at);
        let Ok(id) = RecordingId::derive(captured_at, offset, &staged.digest) else {
            return self.defer_staged(
                run,
                super::deferral::Deferral {
                    candidate,
                    seen,
                    reason: vpt_domain::sweep::DeferralReason::InvalidContainer,
                },
                &staged.path,
                now,
                mode,
                report,
            );
        };
        let target_name = format!("{id}.m4a");
        let intent_id = self.archive_intent(run, &target_name, now, report)?;
        let (audio_path, duplicate) = match self.archive.publish(&staged, archived, &target_name) {
            Ok(Published::Placed(path)) => (path, false),
            Ok(Published::Exists(path)) => {
                self.validate_existing(&path, &staged, report)?;
                (path, true)
            }
            Err(error @ ArchiveError::PlacedUnsynced(_)) => {
                return Err(Self::error(report, archive_failure(error), None));
            }
            Err(error @ ArchiveError::Changed(_)) => {
                report
                    .log
                    .push("staged cleanup pending: archive changed during publication".into());
                return Err(Self::error(report, archive_failure(error), None));
            }
            Err(error) => {
                return Err(Self::owned_error(
                    report,
                    archive_failure(error),
                    staged.path.clone(),
                ));
            }
        };
        let (title, title_source) = match (&mode.import, self.recorder.title(&candidate.file_name))
        {
            (Some(import), _) => (Some(import.title.clone()), TitleOrigin::Import),
            (None, TitleLookup::Titled(title)) => (Some(title), TitleOrigin::VoiceMemos),
            (None, TitleLookup::Unavailable) => (None, TitleOrigin::Unavailable),
        };
        let record = RecordingRecord {
            id: id.clone(),
            source_path: Some(mode.source(candidate).to_path_buf()),
            digest: staged.digest,
            captured_at,
            captured_offset: offset,
            duration_secs: container.duration_secs,
            title,
            title_source,
            ingested_at: now,
            audio_path,
            stages: StageStates::fresh(),
            audio_trashed_at: None,
        };
        self.commit_record(
            run,
            candidate,
            seen,
            now,
            Accepted {
                record,
                existing: false,
                intent: Some(intent_id),
                source: mode.source(candidate).to_path_buf(),
            },
            report,
        )?;
        if duplicate {
            report.ingested.retain(|record| record.id != id);
            report.already_ingested.push(id.clone());
            report.recovered.push(id);
            report.skipped += 1;
            self.discard_owned(&staged.path, report);
        }
        Ok(())
    }
}
