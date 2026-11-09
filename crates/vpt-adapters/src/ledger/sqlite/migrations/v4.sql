CREATE TABLE owned_notes (
  target TEXT PRIMARY KEY CHECK(length(CAST(target AS BLOB)) <= 65536),
  recording TEXT NOT NULL REFERENCES recordings(id),
  stage TEXT NOT NULL CHECK(stage IN ('transcript', 'analysis')),
  digest TEXT NOT NULL CHECK(length(digest) = 64),
  content_digest TEXT NOT NULL CHECK(length(content_digest) = 64),
  body TEXT NOT NULL CHECK(length(CAST(body AS BLOB)) <= 67108864),
  managed_tags TEXT NOT NULL CHECK(length(CAST(managed_tags AS BLOB)) <= 16777216),
  first_written INTEGER NOT NULL,
  write_offset INTEGER NOT NULL CHECK(write_offset > -86400 AND write_offset < 86400),
  updated_at INTEGER NOT NULL,
  UNIQUE(recording, stage)
);
