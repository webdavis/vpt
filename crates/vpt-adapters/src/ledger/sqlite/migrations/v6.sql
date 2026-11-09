ALTER TABLE recordings ADD COLUMN tags_initialized INTEGER NOT NULL DEFAULT 0 CHECK (tags_initialized IN (0, 1));
ALTER TABLE tags ADD COLUMN updated_at INTEGER;
ALTER TABLE tags ADD COLUMN ordinal INTEGER NOT NULL DEFAULT 0;
WITH ordered AS (
    SELECT recording, tag, row_number() OVER (PARTITION BY recording ORDER BY tag) - 1 AS position
    FROM tags
)
UPDATE tags SET ordinal = (
    SELECT position FROM ordered WHERE ordered.recording = tags.recording AND ordered.tag = tags.tag
);
CREATE UNIQUE INDEX tags_order ON tags(recording, ordinal);
UPDATE recordings SET tags_initialized = 1 WHERE EXISTS (SELECT 1 FROM tags WHERE tags.recording = recordings.id);
