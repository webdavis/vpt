ALTER TABLE releases ADD COLUMN report_digest TEXT;
CREATE TABLE release_payloads (
    sequence INTEGER PRIMARY KEY REFERENCES releases(sequence),
    body TEXT NOT NULL,
    report TEXT NOT NULL,
    copy_intent INTEGER NOT NULL REFERENCES history_records(sequence),
    report_intent INTEGER NOT NULL REFERENCES history_records(sequence)
);
CREATE TABLE release_completions (
    sequence INTEGER PRIMARY KEY REFERENCES releases(sequence),
    completed_at INTEGER NOT NULL
);
CREATE INDEX release_recording_sequence ON releases(recording, sequence DESC);
CREATE UNIQUE INDEX release_destination ON releases(destination);
CREATE UNIQUE INDEX release_report_path ON releases(report_path);
