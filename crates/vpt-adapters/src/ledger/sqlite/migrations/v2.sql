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
