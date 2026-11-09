ALTER TABLE known_term_publications ADD COLUMN kind TEXT NOT NULL DEFAULT 'terms'
  CHECK(kind IN ('terms', 'tags'));
