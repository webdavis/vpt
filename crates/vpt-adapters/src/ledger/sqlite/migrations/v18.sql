CREATE TABLE action_sends (
    recording TEXT NOT NULL REFERENCES recordings(id),
    id TEXT NOT NULL CHECK(length(CAST(id AS BLOB)) BETWEEN 1 AND 200),
    digest TEXT NOT NULL CHECK(length(digest) = 64),
    state TEXT NOT NULL CHECK(state IN ('pending', 'sent', 'refused')),
    PRIMARY KEY (recording, id)
);
