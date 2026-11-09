CREATE TABLE artifact_expirations (
    path TEXT PRIMARY KEY NOT NULL CHECK(length(CAST(path AS BLOB)) BETWEEN 1 AND 65536),
    kind TEXT NOT NULL CHECK(kind IN ('engine_output', 'transcript', 'analysis', 'brief', 'redaction_report', 'release')),
    owner TEXT NOT NULL CHECK(length(CAST(owner AS BLOB))=30),
    digest TEXT NOT NULL CHECK(length(CAST(digest AS BLOB))=64),
    trashed_at INTEGER NOT NULL
);
