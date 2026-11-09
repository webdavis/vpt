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
    let long = "A".repeat(80);
    let truncated = "a".repeat(60);
    for (source, limit, slug) in [
        (Some("Ｆｏｏ Straße Kelvin"), None, "foo-strasse-kelvin"),
        (Some("../Hello\\World:_#[]*?"), None, "hello-world"),
        (Some("  Hello---  World  "), None, "hello-world"),
        (Some("cafe\u{301} 世界"), None, "café-世界"),
        (Some(" /.. 😺 "), None, "untitled"),
        (None, None, "untitled"),
        (Some(long.as_str()), None, truncated.as_str()),
        (Some("first second third"), Some(12), "first-second"),
        (Some("世界您好再见"), Some(4), "世界您好"),
    ] {
        let (sandbox, record) = note::configured();
        let extra = limit.map_or(String::new(), |limit| {
            format!("[note]\nslug_max_chars={limit}\n")
        });
        agent(&sandbox, &extra);
        title(&sandbox, record.id.as_str(), source);
        note::success(&invoke(&sandbox, record.id.as_str(), false));
        let before = ledger(&sandbox)
            .note_inputs(&record.id)
            .unwrap()
            .unwrap()
            .notes;
        assert_eq!(before.len(), 2);
        let filename = format!("2026-08-24-{slug}-07070707.md");
        for artifact in &before {
            let store = if artifact.stage == NoteStage::Transcript {
                "transcripts"
            } else {
                "analysis"
            };
            assert_eq!(
                artifact.target,
                sandbox.path().join("home/.vpt").join(store).join(&filename)
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
                record.id.as_str(),
                "--tag",
                "Changed tag",
                "--json",
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
}
