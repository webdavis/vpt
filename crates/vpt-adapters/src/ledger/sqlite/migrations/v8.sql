CREATE TABLE mention_ambiguities (
  recording TEXT NOT NULL REFERENCES recordings(id),
  term TEXT NOT NULL,
  ranges TEXT NOT NULL,
  PRIMARY KEY (recording, term)
);
CREATE INDEX recordings_capture_order ON recordings(captured_at, id);
