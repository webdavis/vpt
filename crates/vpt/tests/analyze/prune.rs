use super::*;
use std::os::unix::fs::MetadataExt;

fn propose(sandbox: &Sandbox, names: &[&str]) {
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
}

fn writing(seeds: &[u8], vocabulary: &str) -> (Sandbox, Vec<RecordingRecord>, std::path::PathBuf) {
    let (sandbox, rows) = note::configured_with(seeds);
    let known = sandbox.path().join("config/vpt/known-tags.txt");
    if !vocabulary.is_empty() {
        fs::write(&known, vocabulary).unwrap();
    }
    agent(
        &sandbox,
        &format!("[tags]\nnew_tags = 'write'\nknown_tags_path = {known:?}\n"),
    );
    (sandbox, rows, known)
}

fn redo(sandbox: &Sandbox, row: &RecordingRecord, names: &[&str]) {
    propose(sandbox, names);
    note::success(&invoke(sandbox, row.id.as_str(), false));
}

#[test]
fn a_learned_tag_nothing_carries_leaves_known_tags_once() {
    let (sandbox, rows, known) = writing(&[7], "");
    redo(&sandbox, &rows[0], &["alpha", "beta"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\nbeta\n");
    redo(&sandbox, &rows[0], &["alpha"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\n");
    let before = fs::metadata(&known).unwrap();
    redo(&sandbox, &rows[0], &["alpha"]);
    let after = fs::metadata(&known).unwrap();
    assert_eq!(
        (before.ino(), before.mtime(), before.mtime_nsec()),
        (after.ino(), after.mtime(), after.mtime_nsec())
    );
}

#[test]
fn a_learned_tag_another_recording_carries_stays() {
    let (sandbox, rows, known) = writing(&[7, 8], "");
    redo(&sandbox, &rows[0], &["alpha", "beta"]);
    redo(&sandbox, &rows[1], &["beta"]);
    redo(&sandbox, &rows[0], &["alpha"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\nbeta\n");
    redo(&sandbox, &rows[1], &[]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\n");
}

#[test]
fn hand_set_tags_stay_at_zero_automatic_uses() {
    let (sandbox, rows, known) = writing(&[7], "handmade\n");
    redo(&sandbox, &rows[0], &["handmade", "beta"]);
    note::success(&support::run_with_budget(
        sandbox
            .vpt()
            .args(["confirm", rows[0].id.as_str(), "--tag", "beta", "--json"]),
        Duration::from_secs(5),
    ));
    redo(&sandbox, &rows[0], &[]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "handmade\nbeta\n");
}

#[test]
fn a_learned_tag_an_indexed_note_carries_stays() {
    let (sandbox, rows, known) = writing(&[7], "");
    redo(&sandbox, &rows[0], &["alpha", "beta"]);
    let external = sandbox.path().join("home/.vpt/transcripts/Owner.md");
    fs::write(&external, "---\ntags: [beta]\n---\n# Owner\n").unwrap();
    redo(&sandbox, &rows[0], &["alpha"]);
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\nbeta\n");
}

#[test]
fn an_interrupted_removal_finishes_on_the_next_run() {
    let (sandbox, rows, known) = writing(&[7], "");
    redo(&sandbox, &rows[0], &["alpha", "beta"]);
    super::workflow::sql(
        &sandbox,
        "CREATE TRIGGER refuse_vocabulary_ack BEFORE DELETE ON dirty_publications WHEN OLD.target LIKE '%/known-tags.txt' BEGIN SELECT RAISE(ABORT, 'private ack'); END",
    );
    propose(&sandbox, &["alpha"]);
    let failed = invoke(&sandbox, rows[0].id.as_str(), false);
    assert_eq!(failed.status.code(), Some(1), "{}", stderr(&failed));
    assert!(!ledger(&sandbox).pending_publications().unwrap().is_empty());
    super::workflow::sql(&sandbox, "DROP TRIGGER refuse_vocabulary_ack");
    fs::write(sandbox.path().join("agent.sh"), "exit 29\n").unwrap();
    note::success(&invoke(&sandbox, rows[0].id.as_str(), false));
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert_eq!(fs::read_to_string(&known).unwrap(), "alpha\n");
}
