use super::*;

fn title(sandbox: &Sandbox, id: &str, value: Option<&str>) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    assert_eq!(
        connection
            .execute(
                "UPDATE recordings SET title=?1 WHERE id=?2",
                rusqlite::params![value, id]
            )
            .unwrap(),
        1
    );
}

#[test]
fn both_flat_note_paths_use_only_capture_title_and_identity_then_remain_pinned() {
    let (sandbox, record) = note::configured();
    agent(&sandbox, "[note]\nslug_max_chars=12\n");
    title(&sandbox, record.id.as_str(), Some("first second third"));
    note::success(&invoke(&sandbox, record.id.as_str(), false));
    let before = ledger(&sandbox)
        .note_inputs(&record.id)
        .unwrap()
        .unwrap()
        .notes;
    assert_eq!(before.len(), 2);
    let filename = "2026-08-24-first-second-07070707.md";
    for artifact in &before {
        let store = if artifact.stage == NoteStage::Transcript {
            "transcripts"
        } else {
            "analysis"
        };
        assert_eq!(
            artifact.target,
            sandbox.path().join("home/.vpt").join(store).join(filename)
        );
        assert!(
            artifact
                .text
                .contains(&format!("# {}\n", filename.strip_suffix(".md").unwrap()))
        );
    }
    title(
        &sandbox,
        record.id.as_str(),
        Some("A completely different title"),
    );
    agent(&sandbox, "[note]\nslug_max_chars=1\n");
    note::success(&support::run_with_budget(
        sandbox.vpt().args([
            "confirm",
            "tag",
            record.id.as_str(),
            "Changed tag",
            "-o",
            "json",
        ]),
        Duration::from_secs(5),
    ));
    let after = ledger(&sandbox)
        .note_inputs(&record.id)
        .unwrap()
        .unwrap()
        .notes;
    for (previous, artifact) in before.iter().zip(after) {
        assert_eq!(artifact.target, previous.target);
        assert_eq!(artifact.first_written, previous.first_written);
        assert_eq!(artifact.content_digest, previous.content_digest);
        assert_eq!(fs::read_to_string(artifact.target).unwrap(), artifact.text);
    }
}
