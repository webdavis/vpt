use super::candidate::{fresh_seen, recorder_failure};
use super::deferral::Deferral;
use super::{Ingest, IngestError, IngestFailure, IngestReport, Mode};
use crate::ports::{
    Archive, ArchiveError, Candidate, Clock, LedgerCommit, Published, RecorderStore,
    RecordingLedger, RecordingRecord, RunHistory, SeenRow, StageStates, TitleLookup, TitleOrigin,
    Trash, WorkDelta,
};
use vpt_domain::container::inspect;
use vpt_domain::history::RunId;
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, HistoryFact, WorkOutcome};
use vpt_domain::sweep::{DeferralReason, SeenFacts, SweepLimits, rest_gate, size_gate};
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
        let source = self
            .recorder
            .open(&candidate.path)
            .map_err(|error| Self::error(report, recorder_failure(error), None))?;
        let before = self
            .recorder
            .metadata(&source)
            .map_err(|error| Self::error(report, recorder_failure(error), None))?;
        let limits = SweepLimits {
            max_audio_bytes: self.settings.max_audio_bytes,
            quiet_period_secs: self.settings.quiet_period_secs,
        };
        if let Err(reason) = size_gate(before.size, &limits) {
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
        if inspect(before.size, |offset, buffer| {
            self.recorder.read_at(&source, offset, buffer)
        })
        .is_err()
        {
            return self.defer(
                run,
                Deferral {
                    candidate,
                    seen,
                    reason: DeferralReason::InvalidContainer,
                },
                now,
                mode,
                report,
            );
        }
        let previous = seen.as_ref().map(|row| SeenFacts {
            size: row.size,
            mtime: row.mtime,
            ingested: row.recording.is_some() && row.deferral_reason.is_none(),
            deferred_size: row.deferred_size,
        });
        if let Err(reason) = rest_gate(before.mtime, before.size, now, previous.as_ref(), &limits) {
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
        let offset = self.clock.offset_at(container.creation_time);
        let id = RecordingId::derive(container.creation_time, offset, &staged.digest);
        let Ok(id) = id else {
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
