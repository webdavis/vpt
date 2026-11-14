CREATE TABLE action_sends_v19 (
    recording TEXT NOT NULL REFERENCES recordings(id),
    id TEXT NOT NULL CHECK(length(CAST(id AS BLOB)) BETWEEN 1 AND 200),
    digest TEXT NOT NULL CHECK(length(digest) = 64),
    state TEXT NOT NULL CHECK(state IN ('pending', 'sent', 'refused', 'withdrawing')),
    attempted INTEGER NOT NULL CHECK(attempted IN (0, 1)),
    PRIMARY KEY (recording, id)
);
INSERT INTO action_sends_v19 (recording, id, digest, state, attempted)
SELECT recording, id, digest, state, 1 FROM action_sends;
DROP TABLE action_sends;
ALTER TABLE action_sends_v19 RENAME TO action_sends;
