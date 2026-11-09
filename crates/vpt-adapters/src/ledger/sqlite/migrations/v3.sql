CREATE TABLE engine_outputs (
  target TEXT PRIMARY KEY CHECK(length(CAST(target AS BLOB)) <= 65536),
  recording TEXT NOT NULL REFERENCES recordings(id),
  engine TEXT NOT NULL CHECK(length(CAST(engine AS BLOB)) BETWEEN 1 AND 64),
  digest TEXT NOT NULL CHECK(length(digest) = 64),
  duration REAL NOT NULL CHECK(duration >= 0),
  language_source TEXT NOT NULL CHECK(language_source IN ('reported', 'requested')),
  body BLOB NOT NULL CHECK(length(body) <= 16777216),
  created_at INTEGER NOT NULL
);
CREATE INDEX engine_outputs_recording ON engine_outputs(recording);
