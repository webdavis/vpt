CREATE TABLE IF NOT EXISTS ledger_identity (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    database_id TEXT NOT NULL CHECK(length(database_id) = 32),
    write_counter INTEGER NOT NULL CHECK(write_counter >= 0)
);
INSERT OR IGNORE INTO ledger_identity (id, database_id, write_counter)
VALUES (1, lower(hex(randomblob(16))), 0);
CREATE TABLE IF NOT EXISTS ledger_writes (
    write_counter INTEGER PRIMARY KEY,
    token TEXT NOT NULL CHECK(length(token) = 32)
);
