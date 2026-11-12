use super::*;
use std::fs;
use vpt_domain::tags::{Provenance, State, Tag};
#[path = "tag_rename/merge.rs"]
mod merge;
#[path = "tag_rename/partial.rs"]
mod partial;

pub(super) fn rename(sandbox: &Sandbox, args: &[&str]) -> std::process::Output {
    support::run_with_budget(
        sandbox
            .vpt()
            .args(["tag", "rename"])
            .args(args)
            .arg("--json"),
        std::time::Duration::from_secs(10),
    )
}

pub(super) fn proposed(name: &str, state: State) -> Tag {
    Tag {
        provenance: Provenance::Proposal {
            by: "summary".into(),
            reason: "stand-in".into(),
            input: vpt_domain::digest::Sha256Digest([3; 32]),
        },
        ..super::tags::tag(name, state)
    }
}

pub(super) fn saved_tags(sandbox: &Sandbox, row: &RecordingRecord) -> Vec<(String, State, bool)> {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .tags(&row.id)
        .unwrap()
        .unwrap()
        .into_iter()
        .map(|tag| (tag.name, tag.state, tag.provenance == Provenance::Manual))
        .collect()
}

pub(super) fn known_tags(sandbox: &Sandbox) -> std::path::PathBuf {
    let path = sandbox.config_path().with_file_name("known-tags.txt");
    fixture::write_config(sandbox, &format!("[tags]\nknown_tags_path={path:?}\n"));
    path
}

#[test]
fn renaming_rewrites_owned_notes_saved_tags_and_known_tags_and_remembers_the_rename() {
    let (sandbox, rows) = configured_with(&[7, 8, 9]);
    let path = known_tags(&sandbox);
    fs::write(&path, "Old_Topic\nkeep\n").unwrap();
    super::tags::update(
        &sandbox,
        &rows[0].id,
        vec![
            proposed("Old_Topic", State::Confirmed),
            proposed("keep", State::Confirmed),
        ],
    );
    super::tags::update(
        &sandbox,
        &rows[1].id,
        vec![super::tags::tag("old-topic", State::Rejected)],
    );
    super::tags::update(
        &sandbox,
        &rows[2].id,
        vec![proposed("keep", State::Confirmed)],
    );
    for row in &rows {
        success(&invoke(&sandbox, row.id.as_str()));
    }
    let untouched = saved(&sandbox, &rows[2]);
    let brief = sandbox.path().join("home/.vpt/briefs/Brief.md");
    fs::create_dir_all(brief.parent().unwrap()).unwrap();
    let brief_text = "---\ntags: [old-topic]\n---\nMeeting brief\n";
    fs::write(&brief, brief_text).unwrap();
    let output = rename(&sandbox, &["old topic", "Fresh Topic"]);
    let result = success(&output);
    assert_eq!(result["command"], "tag rename");
    assert_eq!(result["dry_run"], false);
    assert_eq!(result["changed"], true);
    assert_eq!(result["known_tags_changed"], true);
    assert_eq!(result["untouched_briefs"], 1);
    assert_eq!(result["untouched_notes"], 0);
    assert_eq!(fs::read_to_string(&brief).unwrap(), brief_text);
    let ids: Vec<_> = result["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect();
    let mut expected = vec![rows[0].id.to_string(), rows[1].id.to_string()];
    expected.sort();
    assert_eq!(ids, expected);
    let first = saved(&sandbox, &rows[0]);
    assert_eq!(first.managed_tags, ["fresh-topic", "keep"]);
    assert_eq!(fs::read_to_string(&first.target).unwrap(), first.text);
    let listed = result["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == rows[0].id.as_str())
        .unwrap();
    assert_eq!(listed["notes"][0], first.target.to_str().unwrap());
    assert_eq!(
        saved_tags(&sandbox, &rows[0]),
        [
            ("fresh-topic".to_owned(), State::Confirmed, false),
            ("keep".to_owned(), State::Confirmed, false),
        ]
    );
    assert_eq!(
        saved_tags(&sandbox, &rows[1]),
        [("fresh-topic".to_owned(), State::Rejected, true)]
    );
    assert!(!saved(&sandbox, &rows[1]).text.contains("fresh-topic"));
    assert_eq!(saved(&sandbox, &rows[2]), untouched);
    assert_eq!(fs::read_to_string(&path).unwrap(), "fresh-topic\nkeep\n");
    let renames = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .tag_renames()
        .unwrap();
    assert_eq!(renames.len(), 1);
    assert_eq!(
        (renames[0].old.as_str(), renames[0].new.as_str()),
        ("old-topic", "fresh-topic")
    );
    assert!(!stdout(&output).contains("topic"));
    let again = success(&rename(&sandbox, &["old topic", "Fresh Topic"]));
    assert_eq!(again["changed"], false);
    assert_eq!(again["recordings"], serde_json::json!([]));
}

#[test]
fn malformed_or_equal_names_are_usage_errors_and_an_absent_tag_changes_nothing() {
    let (sandbox, rows) = configured_with(&[7]);
    let path = known_tags(&sandbox);
    fs::write(&path, "keep\n").unwrap();
    for args in [
        &["only-one"][..],
        &[],
        &["#bad", "good"],
        &["good", "123"],
        &["Same_Tag", "same-tag"],
        &["a", "b", "c"],
    ] {
        let output = rename(&sandbox, args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?} {}",
            stderr(&output)
        );
    }
    let output = rename(&sandbox, &["absent", "other"]);
    let result = success(&output);
    assert_eq!(result["changed"], false);
    assert_eq!(result["known_tags_changed"], false);
    assert_eq!(fs::read_to_string(&path).unwrap(), "keep\n");
    assert!(
        SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap()
            .tag_renames()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
            .unwrap()
            .unwrap()
            .tags(&rows[0].id)
            .unwrap(),
        None
    );
}
