use super::*;

fn vault(seeds: &[u8]) -> (Sandbox, Vec<RecordingRecord>, std::path::PathBuf) {
    let (sandbox, rows) = note::configured_with(seeds);
    let known = sandbox.path().join("config/vpt/known-tags.txt");
    let vault = sandbox.path().join("vault");
    let mut stores = String::from("[stores]\n");
    for store in ["transcripts", "analysis", "briefs"] {
        let path = vault.join("vpt").join(store);
        fs::create_dir_all(&path).unwrap();
        stores.push_str(&format!("{store} = {path:?}\n"));
    }
    agent(
        &sandbox,
        &format!(
            "[tags]\nnew_tags = 'write'\nknown_tags_path = {known:?}\n[note]\nprofile = 'obsidian'\n[note.obsidian]\nvault_root = {vault:?}\n{stores}"
        ),
    );
    (sandbox, rows, known)
}

fn redo(sandbox: &Sandbox, row: &RecordingRecord, names: &[&str]) {
    let tags: Vec<_> = names
        .iter()
        .map(|name| serde_json::json!({"tag": name, "reason": "private reason"}))
        .collect();
    let proposal = serde_json::json!({
        "schema": "vpt.proposal/1", "by": "private agent", "summary": [], "actions": [],
        "tags": tags, "relations": []
    });
    fs::write(
        sandbox.path().join("captured.proposal"),
        proposal.to_string(),
    )
    .unwrap();
    note::success(&invoke(sandbox, row.id.as_str(), false));
}

fn vault_notes_carrying(sandbox: &Sandbox, tag: &str) -> usize {
    let analysis = sandbox.path().join("vault/vpt/analysis");
    fs::read_dir(analysis)
        .unwrap()
        .filter(|entry| {
            fs::read_to_string(entry.as_ref().unwrap().path())
                .unwrap()
                .contains(tag)
        })
        .count()
}

#[test]
fn a_learned_tag_only_this_recordings_vault_notes_carried_leaves_known_tags() {
    let (sandbox, rows, known) = vault(&[7]);
    redo(&sandbox, &rows[0], &["alpha", "beta"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\nbeta\n");
    assert_eq!(vault_notes_carrying(&sandbox, "beta"), 1);
    redo(&sandbox, &rows[0], &["alpha"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\n");
}

#[test]
fn a_learned_tag_a_vault_note_outside_vpt_carries_stays() {
    let (sandbox, rows, known) = vault(&[7]);
    redo(&sandbox, &rows[0], &["alpha", "beta"]);
    let external = sandbox.path().join("vault/Owner.md");
    fs::write(&external, "---\ntags: [beta]\n---\n# Owner\n").unwrap();
    redo(&sandbox, &rows[0], &["alpha"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\nbeta\n");
    assert_eq!(
        fs::read_to_string(&external).unwrap(),
        "---\ntags: [beta]\n---\n# Owner\n"
    );
}

#[test]
fn a_learned_tag_another_recordings_vault_notes_carry_stays() {
    let (sandbox, rows, known) = vault(&[7, 8]);
    redo(&sandbox, &rows[0], &["alpha", "beta"]);
    redo(&sandbox, &rows[1], &["beta"]);
    redo(&sandbox, &rows[0], &["alpha"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\nbeta\n");
    redo(&sandbox, &rows[1], &[]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\n");
}
