use crate::ports::{
    Candidate, HistoryReceipt, LedgerError, RecordingRecord, SinkError, SinkWarning, TitleOrigin,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use vpt_domain::container::Container;
use vpt_domain::digest::Sha256Digest;
use vpt_domain::identity::RecordingId;
use vpt_domain::sweep::DeferralReason;
use vpt_domain::time::UtcInstant;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mode {
    pub dry_run: bool,
    pub once: Option<PathBuf>,
    pub import: Option<Import>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    pub origin: PathBuf,
    pub title: String,
    pub captured_at: Option<UtcInstant>,
    pub modified_at: UtcInstant,
}

impl Mode {
    pub(super) fn source<'a>(&'a self, candidate: &'a Candidate) -> &'a Path {
        self.import
            .as_ref()
            .map_or(&candidate.path, |import| &import.origin)
    }

    pub(super) fn captured_at(&self, container: &Container) -> UtcInstant {
        match &self.import {
            None => container.creation_time,
            Some(import) => import
                .captured_at
                .or_else(|| container.recorded_creation_time())
                .unwrap_or(import.modified_at),
        }
    }
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
    pub(super) owned_staging: Option<PathBuf>,
}
