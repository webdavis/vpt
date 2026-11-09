ALTER TABLE verified_artifacts ADD COLUMN claims_digest TEXT
  CHECK (claims_digest IS NULL OR length(claims_digest)=64);
-- Older checks did not retain a claim-set fingerprint; their decisions remain available for rechecking.
UPDATE recordings SET open_flags = (
  SELECT count(*) FROM flags WHERE recording=recordings.id AND state='open'
  AND shape='lexical' AND class!='formatting'
);
