use super::*;

#[test]
fn a_dry_run_lists_the_notes_then_a_merge_keeps_hand_set_state_and_never_writes_vault_notes() {
    let (sandbox, rows) = configured_with(&[7, 8]);
    let path = known_tags(&sandbox);
    let vault = sandbox.path().join("vault");
    fs::create_dir_all(&vault).unwrap();
    let external = vault.join("External.md");
    let original = "---\ntags: [old-topic, mine]\n---\nPrivate external prose\n";
    fs::write(&external, original).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[tags]\nknown_tags_path={path:?}\n[note]\nprofile='obsidian'\n[note.obsidian]\nvault_root={vault:?}\n"
        ),
    );
    fs::write(&path, "old-topic\nnew-topic\n").unwrap();
    super::super::tags::update(
        &sandbox,
        &rows[0].id,
        vec![
            proposed("new-topic", State::Confirmed),
            super::super::tags::tag("old-topic", State::Rejected),
        ],
    );
    super::super::tags::update(
        &sandbox,
        &rows[1].id,
        vec![super::super::tags::tag("old-topic", State::Confirmed)],
    );
    for row in &rows {
        success(&invoke(&sandbox, row.id.as_str()));
    }
    let notes: Vec<_> = rows.iter().map(|row| saved(&sandbox, row)).collect();
    let preview = success(&rename(&sandbox, &["old-topic", "new-topic", "--dry-run"]));
    assert_eq!(preview["dry_run"], true);
    assert_eq!(preview["changed"], true);
    assert_eq!(preview["known_tags_changed"], true);
    assert_eq!(preview["untouched_notes"], 1);
    assert_eq!(preview["untouched_briefs"], 0);
    let listed: Vec<_> = preview["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["notes"].as_array().unwrap().clone())
        .collect();
    assert_eq!(listed.len(), 2);
    for note in &notes {
        assert!(listed.contains(&serde_json::json!(note.target)));
        assert_eq!(fs::read_to_string(&note.target).unwrap(), note.text);
    }
    assert_eq!(fs::read_to_string(&path).unwrap(), "old-topic\nnew-topic\n");
    rows.iter().for_each(|row| {
        assert!(
            saved_tags(&sandbox, row)
                .iter()
                .any(|tag| tag.0 == "old-topic")
        )
    });
    let result = success(&rename(&sandbox, &["old-topic", "new-topic"]));
    assert_eq!(result["untouched_notes"], 1);
    assert_eq!(
        saved_tags(&sandbox, &rows[0]),
        [("new-topic".to_owned(), State::Rejected, true)]
    );
    assert_eq!(
        saved_tags(&sandbox, &rows[1]),
        [("new-topic".to_owned(), State::Confirmed, true)]
    );
    assert!(saved(&sandbox, &rows[0]).managed_tags.is_empty());
    assert_eq!(saved(&sandbox, &rows[1]).managed_tags, ["new-topic"]);
    assert_eq!(fs::read_to_string(&path).unwrap(), "new-topic\n");
    assert_eq!(fs::read_to_string(&external).unwrap(), original);
}
