use super::candidate::{fresh_seen, recorder_failure};
use super::{Ingest, IngestError, IngestFailure, IngestReport};
use crate::ports::{
    Archive, ArchiveError, Candidate, Clock, LedgerCommit, Published, RecorderStore,
    RecordingLedger, RecordingRecord, RunHistory, SeenRow, StageStates, TitleLookup, TitleOrigin,
    Trash, TrashError, WorkDelta,
};
use vpt_domain::container::inspect;
use vpt_domain::history::RunId;
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, HistoryFact, WorkOutcome};
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
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let source = self
            .recorder
            .open(&candidate.path)
            .map_err(|error| Self::error(report, recorder_failure(error), None))?;
        let before = self
            .recorder
            .metadata(&source)
            .map_err(|error| Self::error(report, recorder_failure(error), None))?;
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
            return Err(Self::owned_error(
                report,
                IngestFailure::Recorder("source changed during staging".into()),
                staged.path.clone(),
            ));
        }
        let archived = self.archive.open(&staged.path).map_err(|error| {
            Self::owned_error(report, archive_failure(error), staged.path.clone())
        })?;
        let size = self.archive.size(&archived).map_err(|error| {
            Self::owned_error(report, archive_failure(error), staged.path.clone())
        })?;
        let container = inspect(size, |offset, buffer| {
            self.archive.read_at(&archived, offset, buffer)
        })
        .map_err(|_| {
            Self::owned_error(
                report,
                IngestFailure::Archive("staged container invalid".into()),
                staged.path.clone(),
            )
        })?;
        let offset = self.clock.offset_at(container.creation_time);
        let id =
            RecordingId::derive(container.creation_time, offset, &staged.digest).map_err(|_| {
                Self::owned_error(
                    report,
                    IngestFailure::Archive("capture instant invalid".into()),
                    staged.path.clone(),
                )
            })?;
        let target_name = format!("{id}.m4a");
        let intent = Self::required(
            self.ledger.commit_step(
                run,
                &WorkDelta::default(),
                &[(
                    now,
                    HistoryFact::EffectIntent {
                        kind: EffectKind::Archive,
                        reference: EffectReference::ArchiveTarget(
                            self.archive.target(&target_name),
                        ),
                    },
                )],
            ),
            report,
        )?;
        self.export(&intent, report)?;
        let intent_id = intent
            .records
            .first()
            .map(|record| record.id)
            .ok_or_else(|| {
                Self::error(
                    report,
                    IngestFailure::RequiredHistory(crate::ports::LedgerError::Corrupt(
                        "archive intent receipt missing".into(),
                    )),
                    None,
                )
            })?;
        let audio_path = match self.archive.publish(&staged.path, &target_name) {
            Ok(Published::Placed(path)) => path,
            Ok(Published::Exists(path)) => {
                return Err(Self::owned_error(
                    report,
                    IngestFailure::Archive(format!("archive target exists at {}", path.display())),
                    staged.path.clone(),
                ));
            }
            Err(error @ ArchiveError::PlacedUnsynced(_)) => {
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
        let (title, title_source) = match self.recorder.title(&candidate.file_name) {
            TitleLookup::Titled(title) => (Some(title), TitleOrigin::VoiceMemos),
            TitleLookup::Unavailable => (None, TitleOrigin::Unavailable),
        };
        let record = RecordingRecord {
            id: id.clone(),
            source_path: Some(candidate.path.clone()),
            digest: staged.digest,
            captured_at: container.creation_time,
            captured_offset: offset,
            duration_secs: container.duration_secs,
            title,
            title_source,
            ingested_at: now,
            audio_path,
            stages: StageStates::fresh(),
            audio_trashed_at: None,
        };
        let mut seen = seen.unwrap_or_else(|| fresh_seen(candidate, now));
        seen.size = candidate.size;
        seen.mtime = candidate.mtime;
        seen.flags = candidate.flags;
        seen.last_seen = now;
        seen.deferral_count = 0;
        seen.deferral_reason = None;
        seen.deferred_size = None;
        seen.source_gone_at = None;
        seen.recording = Some(id.clone());
        let delta = WorkDelta {
            commit: LedgerCommit {
                recordings: vec![record.clone()],
                seen: vec![seen],
                publications: Vec::new(),
            },
            ..Default::default()
        };
        let receipt = Self::required(
            self.ledger.commit_step(
                run,
                &delta,
                &[
                    (
                        now,
                        HistoryFact::EffectObserved {
                            intent: intent_id,
                            outcome: EffectOutcome::Completed,
                        },
                    ),
                    (
                        now,
                        HistoryFact::WorkStep {
                            recording: Some(id),
                            outcome: WorkOutcome::Completed,
                        },
                    ),
                ],
            ),
            report,
        )?;
        self.export(&receipt, report)?;
        report.ingested.push(record);
        Ok(())
    }

    pub(super) fn discard_owned(&self, staged: &std::path::Path, report: &mut IngestReport) {
        if self.archive.validate_trash_path(staged).is_err() {
            report
                .log
                .push("staged cleanup pending: ownership validation failed".into());
            return;
        }
        let outcome = self.trash.trash(staged);
        if !self.trash.take_diagnostics().is_empty() {
            report.log.push("trash reported a diagnostic".into());
        }
        match outcome {
            Ok(_) => {}
            Err(TrashError::HelperAbsent) => report
                .log
                .push("staged cleanup pending: helper unavailable".into()),
            Err(TrashError::HelperVersion { found }) => report.log.push(format!(
                "staged cleanup pending: incompatible helper major {found}"
            )),
            Err(TrashError::Failed(_)) => report
                .log
                .push("staged cleanup pending: helper failed".into()),
            Err(TrashError::Unknown(_)) => report
                .log
                .push("staged cleanup pending: helper reply unknown".into()),
        }
    }
}

fn archive_failure(error: ArchiveError) -> IngestFailure {
    match error {
        ArchiveError::NoSpace => IngestFailure::NoSpace,
        ArchiveError::Sync(detail) => IngestFailure::Sync(detail),
        ArchiveError::PlacedUnsynced(path) => IngestFailure::Sync(format!(
            "archive placed without directory sync at {}",
            path.display()
        )),
        ArchiveError::Escape(path) => IngestFailure::PathEscape(path),
        ArchiveError::Io(detail) => IngestFailure::Archive(detail),
    }
}
