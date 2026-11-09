CREATE TABLE owned_briefs (
    occasion TEXT PRIMARY KEY NOT NULL REFERENCES occasions(id),
    target TEXT UNIQUE NOT NULL,
    digest TEXT NOT NULL,
    snapshot_digest TEXT NOT NULL,
    body TEXT NOT NULL,
    managed_tags TEXT NOT NULL,
    first_written INTEGER NOT NULL,
    write_offset INTEGER NOT NULL,
    published_at INTEGER
);
