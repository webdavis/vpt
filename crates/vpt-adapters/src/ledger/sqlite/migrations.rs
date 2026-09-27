use rusqlite::{Connection, Transaction, TransactionBehavior};
use vpt_application::ports::LedgerError;

pub const VERSION: u32 = 2;

pub(super) fn current(connection: &Connection) -> Result<u32, LedgerError> {
    let version = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(super::map)?;
    if version > VERSION {
        return Err(LedgerError::UnsupportedSchema(version));
    }
    Ok(version)
}

pub(super) fn migrate(
    connection: &Connection,
    before_commit: impl FnOnce() -> Result<(), LedgerError>,
) -> Result<(), LedgerError> {
    if current(connection)? == VERSION {
        return Ok(());
    }
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Immediate)
        .map_err(super::map)?;
    let version = current(&transaction)?;
    if version == VERSION {
        return Ok(());
    }
    if version == 0 {
        transaction.execute_batch(SCHEMA_V1).map_err(super::map)?;
    }
    transaction.execute_batch(SCHEMA_V2).map_err(super::map)?;
    transaction
        .pragma_update(None, "user_version", VERSION)
        .map_err(super::map)?;
    before_commit()?;
    transaction.commit().map_err(super::map)
}

pub(super) const SCHEMA_V1: &str = "
CREATE TABLE seen (
  path TEXT PRIMARY KEY,
  file_name TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime_secs INTEGER NOT NULL,
  mtime_nanos INTEGER NOT NULL,
  flags INTEGER NOT NULL,
  first_seen INTEGER NOT NULL,
  last_seen INTEGER NOT NULL,
  deferral_count INTEGER NOT NULL DEFAULT 0,
  deferral_reason TEXT,
  deferred_size INTEGER,
  source_gone_at INTEGER,
  recording TEXT
);
CREATE TABLE recordings (
  id TEXT PRIMARY KEY,
  source_path TEXT,
  digest TEXT NOT NULL UNIQUE,
  captured_at INTEGER NOT NULL,
  captured_offset INTEGER NOT NULL,
  duration_secs INTEGER NOT NULL,
  title TEXT,
  title_source TEXT NOT NULL,
  ingested_at INTEGER NOT NULL,
  audio_path TEXT NOT NULL,
  stage_transcribe TEXT NOT NULL,
  stage_note TEXT NOT NULL,
  stage_synthesis TEXT NOT NULL,
  transcript_note_path TEXT,
  analysis_note_path TEXT,
  engines TEXT,
  language TEXT,
  open_flags INTEGER NOT NULL DEFAULT 0,
  diagnostics INTEGER NOT NULL DEFAULT 0,
  audio_trashed_at INTEGER
);
CREATE TABLE transcripts (
  recording TEXT PRIMARY KEY REFERENCES recordings(id),
  engine TEXT NOT NULL,
  body TEXT NOT NULL,
  accepted_at INTEGER NOT NULL
);
CREATE TABLE proposals (
  recording TEXT PRIMARY KEY REFERENCES recordings(id),
  input_digest TEXT NOT NULL,
  body TEXT NOT NULL,
  accepted_at INTEGER NOT NULL
);
CREATE TABLE flags (
  identifier TEXT PRIMARY KEY,
  recording TEXT NOT NULL REFERENCES recordings(id),
  shape TEXT NOT NULL,
  class TEXT NOT NULL,
  ranges TEXT,
  artifact TEXT,
  claim_index INTEGER,
  claim_digest TEXT,
  record_text TEXT,
  alternative_text TEXT,
  confidence REAL,
  state TEXT NOT NULL,
  resolution_text TEXT,
  resolved_at INTEGER
);
CREATE TABLE occasions (
  id TEXT PRIMARY KEY,
  provider_key TEXT NOT NULL UNIQUE,
  source TEXT NOT NULL,
  at INTEGER NOT NULL,
  duration_secs INTEGER,
  title TEXT NOT NULL,
  participants TEXT NOT NULL,
  tags TEXT NOT NULL,
  pack TEXT,
  brief_path TEXT,
  briefed_at INTEGER
);
CREATE TABLE tags (
  recording TEXT NOT NULL REFERENCES recordings(id),
  tag TEXT NOT NULL,
  state TEXT NOT NULL,
  provenance TEXT NOT NULL,
  PRIMARY KEY (recording, tag)
);
CREATE TABLE relations (
  recording TEXT NOT NULL REFERENCES recordings(id),
  kind TEXT NOT NULL,
  target TEXT NOT NULL,
  state TEXT NOT NULL,
  provenance TEXT NOT NULL,
  rule TEXT,
  ranges TEXT,
  PRIMARY KEY (recording, kind, target)
);
CREATE TABLE releases (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  recording TEXT NOT NULL REFERENCES recordings(id),
  stage TEXT NOT NULL,
  destination TEXT NOT NULL,
  content_digest TEXT NOT NULL,
  source_digest TEXT NOT NULL,
  report_path TEXT NOT NULL,
  snapshot TEXT NOT NULL,
  released_at INTEGER NOT NULL
);
CREATE TABLE dirty_publications (
  target TEXT PRIMARY KEY,
  expected_previous TEXT,
  intended TEXT NOT NULL,
  recorded_at INTEGER NOT NULL
);
CREATE TABLE retention_intents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  artifact_kind TEXT NOT NULL,
  recording TEXT,
  path TEXT NOT NULL,
  expected TEXT NOT NULL,
  recorded_at INTEGER NOT NULL,
  completed_at INTEGER
);
";

const SCHEMA_V2: &str = "
CREATE TABLE history_identity (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  namespace BLOB NOT NULL CHECK (length(namespace) = 16),
  cursor_generation INTEGER,
  cursor_device TEXT,
  cursor_inode TEXT,
  cursor_end TEXT
);
INSERT INTO history_identity (id, namespace) VALUES (1, randomblob(16));
CREATE TABLE history_runs (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  started_at INTEGER NOT NULL
);
CREATE TABLE history_records (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  run_sequence INTEGER NOT NULL REFERENCES history_runs(sequence),
  occurred_at INTEGER NOT NULL,
  fact TEXT NOT NULL,
  appended_at INTEGER,
  export_device TEXT,
  export_inode TEXT,
  export_end TEXT,
  CHECK (
    (appended_at IS NULL AND export_device IS NULL AND export_inode IS NULL AND export_end IS NULL)
    OR
    (appended_at IS NOT NULL AND export_device IS NOT NULL AND export_inode IS NOT NULL AND export_end IS NOT NULL)
  )
);
CREATE TABLE history_export_failures (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  event_sequence INTEGER NOT NULL REFERENCES history_records(sequence),
  observed_at INTEGER NOT NULL,
  class TEXT NOT NULL,
  last_device TEXT,
  last_inode TEXT,
  last_end TEXT,
  observed_file_json TEXT,
  partial_json TEXT,
  CHECK (
    (last_device IS NULL AND last_inode IS NULL AND last_end IS NULL)
    OR
    (last_device IS NOT NULL AND last_inode IS NOT NULL AND last_end IS NOT NULL)
  )
);
CREATE TABLE history_export_restarts (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  old_generation INTEGER NOT NULL,
  old_device TEXT NOT NULL,
  old_inode TEXT NOT NULL,
  old_end TEXT NOT NULL,
  new_generation INTEGER NOT NULL,
  new_device TEXT NOT NULL,
  new_inode TEXT NOT NULL,
  new_end TEXT NOT NULL,
  observed_at INTEGER NOT NULL
);
";
