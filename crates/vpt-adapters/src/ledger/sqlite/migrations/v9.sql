CREATE TABLE verified_artifacts (
  recording TEXT NOT NULL REFERENCES recordings(id),
  artifact TEXT NOT NULL CHECK (artifact IN ('transcript','analysis')),
  input_digest TEXT NOT NULL CHECK (length(input_digest)=64),
  transcript_digest TEXT NOT NULL CHECK (length(transcript_digest)=64),
  lexical_digest TEXT NOT NULL CHECK (length(lexical_digest)=64),
  checked_at INTEGER NOT NULL,
  PRIMARY KEY (recording, artifact)
);
-- Previous versions did not retain the checked inputs, so old findings await rechecking.
UPDATE recordings SET open_flags = (
  SELECT count(*) FROM flags WHERE recording=recordings.id AND state='open'
  AND shape='lexical' AND class!='formatting'
);
