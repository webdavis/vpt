ALTER TABLE verified_artifacts ADD COLUMN grounding_window_secs INTEGER
 CHECK(grounding_window_secs IS NULL OR grounding_window_secs BETWEEN 0 AND 4294967295);
-- Earlier checks did not save the window; retain their decisions for rechecking.
UPDATE recordings SET open_flags = (
  SELECT count(*) FROM flags WHERE recording=recordings.id AND state='open'
  AND shape='lexical' AND class!='formatting'
);
