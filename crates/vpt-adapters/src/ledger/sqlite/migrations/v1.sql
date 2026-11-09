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
