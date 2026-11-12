pub(super) fn sql(version: u32) -> String {
    let mut sources = vec![
        "SELECT 'audio' AS kind, id AS owner, audio_path AS path, digest, audio_trashed_at IS NOT NULL AS expired, 0 AS pending, audio_trashed_at AS trashed_at FROM recordings",
    ];
    if version >= 3 {
        sources.push(
            "SELECT 'engine_output', recording, target, digest, 0, 0, NULL FROM engine_outputs",
        );
    }
    if version >= 4 {
        sources.push("SELECT CASE n.stage WHEN 'transcript' THEN 'transcript' WHEN 'analysis' THEN 'analysis' END, n.recording, n.target, n.digest,
            CASE WHEN r.id IS NULL
                OR (n.stage='transcript' AND n.target IS NOT r.transcript_note_path)
                OR (n.stage='analysis' AND n.target IS NOT r.analysis_note_path) THEN NULL
                WHEN n.stage='transcript' THEN r.stage_note='expired' ELSE r.stage_analysis='expired' END, 0, NULL
            FROM owned_notes n LEFT JOIN recordings r ON r.id=n.recording");
    }
    if version >= 13 {
        sources.push(
            "SELECT 'brief', b.occasion, b.target, b.digest,
            CASE WHEN o.id IS NULL OR b.target IS NOT o.brief_path THEN NULL ELSE 0 END,
            b.published_at IS NULL, NULL FROM owned_briefs b LEFT JOIN occasions o ON o.id=b.occasion",
        );
    }
    if version >= 14 {
        sources.push("SELECT 'release', r.recording, r.destination, r.content_digest, 0, c.sequence IS NULL, NULL FROM releases r LEFT JOIN release_completions c ON c.sequence=r.sequence");
        sources.push("SELECT 'redaction_report', r.recording, r.report_path, r.report_digest, 0, c.sequence IS NULL, NULL FROM releases r LEFT JOIN release_completions c ON c.sequence=r.sequence");
    }
    format!(
        "SELECT kind,
        CASE WHEN length(CAST(owner AS BLOB))=30 THEN owner END,
        CASE WHEN length(CAST(path AS BLOB))<=65536 THEN path END,
        CASE WHEN length(CAST(digest AS BLOB))=64 THEN digest END,
        CASE WHEN expired IS NULL THEN NULL
             WHEN pending OR EXISTS(SELECT 1 FROM dirty_publications WHERE target=path) THEN 1
             WHEN expired THEN 2 ELSE 0 END, trashed_at
        FROM ({}) LIMIT ?1",
        sources.join(" UNION ALL ")
    )
}
