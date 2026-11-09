use super::{Ingest, IngestError, IngestFailure, IngestReport, candidate::fresh_seen};
use crate::ports::{
    Archive, Candidate, Clock, EventSink, LedgerCommit, RecorderStore, RecordingLedger,
    RecordingRecord, RunHistory, SeenRow, Trash, WorkDelta,
};
use vpt_domain::history::{EventId, RunId};
use vpt_domain::run::{EffectKind, EffectOutcome, EffectReference, HistoryFact, WorkOutcome};
use vpt_domain::time::UtcInstant;

pub(super) struct Accepted {
    pub record: RecordingRecord,
    pub existing: bool,
    pub intent: Option<EventId>,
}

impl<R, A, L, C, T, S> Ingest<'_, R, A, L, C, T, S>
where
    R: RecorderStore,
    A: Archive,
    L: RecordingLedger + RunHistory,
    C: Clock,
    T: Trash + ?Sized,
    S: EventSink + ?Sized,
{
    pub(super) fn archive_intent(
        &mut self,
        run: RunId,
        target_name: &str,
        now: UtcInstant,
        report: &mut IngestReport,
    ) -> Result<EventId, Box<IngestError>> {
        let intent = Self::required(
            self.ledger.commit_step(
                run,
                &WorkDelta::default(),
                &[(
                    now,
                    HistoryFact::EffectIntent {
                        kind: EffectKind::Archive,
                        reference: EffectReference::ArchiveTarget(self.archive.target(target_name)),
                    },
                )],
            ),
            report,
        )?;
        self.export(&intent, report)?;
        intent
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
            })
    }

    pub(super) fn commit_record(
        &mut self,
        run: RunId,
        candidate: &Candidate,
        seen: Option<SeenRow>,
        now: UtcInstant,
        accepted: Accepted,
        report: &mut IngestReport,
    ) -> Result<(), Box<IngestError>> {
        let Accepted {
            record,
            existing,
            intent,
        } = accepted;
        let id = record.id.clone();
        let first_seen = seen.map_or(now, |row| row.first_seen);
        let mut seen = fresh_seen(candidate, now);
        seen.first_seen = first_seen;
        seen.recording = Some(id.clone());
        let mut delta = WorkDelta {
            commit: LedgerCommit {
                seen: vec![seen],
                ..Default::default()
            },
            ..Default::default()
        };
        if existing {
            delta
                .source_updates
                .push((id.clone(), candidate.path.clone()));
        } else {
            delta.commit.recordings.push(record.clone());
        }
        let mut facts = Vec::new();
        if let Some(intent) = intent {
            facts.push((
                now,
                HistoryFact::EffectObserved {
                    intent,
                    outcome: EffectOutcome::Completed,
                },
            ));
        }
        facts.push((
            now,
            HistoryFact::WorkStep {
                recording: Some(id.clone()),
                outcome: WorkOutcome::Completed,
            },
        ));
        let receipt = Self::required(self.ledger.commit_step(run, &delta, &facts), report)?;
        if existing {
            report.already_ingested.push(id);
            report.skipped += 1;
        } else {
            report.ingested.push(record);
        }
        self.export(&receipt, report)
    }
}
