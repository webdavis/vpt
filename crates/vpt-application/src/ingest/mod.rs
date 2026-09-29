mod candidate;
mod deferral;
mod failure;
mod publish;

use crate::SourceSettings;
use crate::ports::{
    Archive, Clock, EventSink, HistoryReceipt, LedgerError, RecorderStore, RecordingLedger,
    RecordingRecord, RunHistory, SinkError, SinkWarning, TitleOrigin, Trash, WorkDelta,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::run::{HistoryFact, RunKind, RunOutcome};
use vpt_domain::sweep::DeferralReason;

pub struct Ingest<'a, R, A, L, C, T: ?Sized, S: ?Sized> {
    pub recorder: &'a R,
    pub archive: &'a A,
    pub ledger: &'a L,
    pub clock: &'a C,
    pub trash: &'a T,
    pub sink: &'a mut S,
    pub settings: &'a SourceSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mode {
    pub dry_run: bool,
    pub once: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deferred {
    pub path: PathBuf,
    pub reason: DeferralReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WouldIngest {
    pub path: PathBuf,
    pub title: Option<String>,
    pub title_source: TitleOrigin,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IngestReport {
    pub ingested: Vec<RecordingRecord>,
    pub already_ingested: Vec<RecordingId>,
    pub recovered: Vec<RecordingId>,
    pub deferred: Vec<Deferred>,
    pub skipped: u64,
    pub would_ingest: Vec<WouldIngest>,
    pub warnings: BTreeSet<SinkWarning>,
    pub log: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IngestFailure {
    DryRunUnavailable,
    StoreUnreadable(String),
    EmptyStore,
    NoSpace,
    Sync(String),
    ArchiveCollision {
        target: PathBuf,
        staged: Sha256Digest,
        existing: Sha256Digest,
    },
    OnceMissing(PathBuf),
    PathEscape(PathBuf),
    HelperVersion {
        found: u32,
    },
    Ledger(LedgerError),
    RequiredHistory(LedgerError),
    SinkStopped(SinkError),
    Archive(String),
    Recorder(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestError {
    pub failure: IngestFailure,
    pub completed: Vec<RecordingId>,
    pub known_receipt: Option<HistoryReceipt>,
    pub warnings: BTreeSet<SinkWarning>,
    pub log: Vec<String>,
    owned_staging: Option<PathBuf>,
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
    pub fn run(&mut self, mode: &Mode) -> Result<IngestReport, Box<IngestError>> {
        let mut report = IngestReport::default();
        if mode.dry_run {
            return Err(Self::error(&report, IngestFailure::DryRunUnavailable, None));
        }
        let started_at = self.clock.now();
        let started = Self::required(self.ledger.begin_run(RunKind::Ingest, started_at), &report)?;
        self.export(&started, &mut report)?;
        if let Err(error) = self.recorder.refresh_titles() {
            let failure = Self::error(&report, candidate::recorder_failure(error), None);
            return Err(self.finish_failure(started.run, self.clock.now(), failure, &mut report));
        }
        let candidates = match &mode.once {
            Some(path) => self
                .recorder
                .candidate(path)
                .map(|candidate| vec![candidate]),
            None => self.recorder.candidates(),
        };
        let candidates = match candidates {
            Ok(candidates) => candidates,
            Err(error) => {
                let failure = Self::error(&report, candidate::recorder_failure(error), None);
                return Err(self.finish_failure(
                    started.run,
                    self.clock.now(),
                    failure,
                    &mut report,
                ));
            }
        };
        for candidate in &candidates {
            let now = self.clock.now();
            if let Err(error) =
                self.process_candidate(started.run, candidate, now, mode, &mut report)
            {
                return Err(self.finish_failure(started.run, now, error, &mut report));
            }
        }
        let finished = Self::required(
            self.ledger.commit_step(
                started.run,
                &WorkDelta::default(),
                &[(
                    self.clock.now(),
                    HistoryFact::RunFinished(RunOutcome::Completed),
                )],
            ),
            &report,
        )?;
        self.export(&finished, &mut report)?;
        Ok(report)
    }
}
