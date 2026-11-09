CREATE TABLE known_term_publications (
  target TEXT NOT NULL PRIMARY KEY REFERENCES dirty_publications(target) ON DELETE CASCADE
    CHECK(length(CAST(target AS BLOB)) BETWEEN 1 AND 65536),
  body BLOB NOT NULL CHECK(typeof(body) = 'blob' AND length(body) BETWEEN 1 AND 16777216)
);
