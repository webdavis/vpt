use super::*;
use std::fs;
use vpt_domain::tags::{Provenance, State};
#[path = "tag_confirm/boundaries.rs"]
mod boundaries;
#[path = "tag_confirm/durability.rs"]
mod durability;

#[test]
fn configured_profile_vocabulary_preserves_external_spelling_and_never_edits_external_notes() {
    for obsidian in [false, true] {
        let (sandbox, row, path) = configured_tags();
        let root = if obsidian {
            sandbox.path().join("vault")
        } else {
            sandbox.path().join("home/.vpt/briefs")
        };
        fs::create_dir_all(&root).unwrap();
        let external = root.join("External.md");
        let original = "---\ntags: [Project_Finance]\n---\nPrivate external prose\n";
        fs::write(&external, original).unwrap();
        if obsidian {
            fixture::write_config(
                &sandbox,
                &format!(
                    "[tags]\nknown_tags_path={path:?}\n[note]\nprofile='obsidian'\n[note.obsidian]\nvault_root={root:?}\n"
                ),
            );
        }
        success(&confirm(
            &sandbox,
            row.id.as_str(),
            "--tag",
            "project/finance",
        ));
        assert_eq!(saved(&sandbox, &row).managed_tags, ["Project_Finance"]);
        assert_eq!(fs::read_to_string(&path).unwrap(), "Project_Finance\n");
        assert_eq!(fs::read_to_string(&external).unwrap(), original);
    }
}

fn confirm(sandbox: &Sandbox, id: &str, flag: &str, tag: &str) -> std::process::Output {
    support::run_with_budget(
        sandbox.vpt().args(["confirm", id, flag, tag, "--json"]),
        std::time::Duration::from_secs(5),
    )
}

fn tags(sandbox: &Sandbox, row: &RecordingRecord) -> Vec<vpt_domain::tags::Tag> {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
        .tags(&row.id)
        .unwrap()
        .unwrap()
}

fn configured_tags() -> (Sandbox, RecordingRecord, std::path::PathBuf) {
    let (sandbox, row) = configured();
    let path = sandbox.config_path().with_file_name("known-tags.txt");
    fixture::write_config(&sandbox, &format!("[tags]\nknown_tags_path={path:?}\n"));
    (sandbox, row, path)
}

#[test]
fn held_tags_can_be_confirmed_or_rejected_without_losing_other_suggestions() {
    let (sandbox, row, path) = configured_tags();
    super::tags::update(
        &sandbox,
        &row.id,
        vec![
            super::tags::tag("First_Topic", State::Suggested),
            super::tags::tag("second-topic", State::Suggested),
            super::tags::tag("third-topic", State::Suggested),
        ],
    );
    success(&invoke(&sandbox, row.id.as_str()));
    success(&confirm(&sandbox, row.id.as_str(), "--tag", "FIRST-TOPIC"));
    success(&confirm(
        &sandbox,
        row.id.as_str(),
        "--reject-tag",
        "second/topic",
    ));
    let note = saved(&sandbox, &row);
    assert_eq!(note.managed_tags, ["First_Topic"]);
    assert!(note.text.contains("third-topic"));
    assert!(!note.text.contains("second-topic"));
    assert_eq!(fs::read_to_string(path).unwrap(), "First_Topic\n");
    assert_eq!(
        tags(&sandbox, &row)
            .iter()
            .map(|tag| tag.state)
            .collect::<Vec<_>>(),
        [State::Confirmed, State::Rejected, State::Suggested]
    );
}

#[test]
fn manual_tag_confirmation_writes_state_vocabulary_and_note_without_analysis() {
    let (sandbox, row, path) = configured_tags();
    let output = confirm(&sandbox, row.id.as_str(), "--tag", "Private topic");
    let result = success(&output);
    assert_eq!(result["record"]["id"], row.id.as_str());
    assert_eq!(result["confirmed"]["tag"], true);
    let note = saved(&sandbox, &row);
    assert_eq!(note.managed_tags, ["private-topic"]);
    assert_eq!(fs::read_to_string(&path).unwrap(), "private-topic\n");
    let first = tags(&sandbox, &row);
    assert_eq!(first[0].state, State::Confirmed);
    assert_eq!(first[0].provenance, Provenance::Manual);
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    success(&confirm(
        &sandbox,
        row.id.as_str(),
        "--tag",
        "PRIVATE_TOPIC",
    ));
    assert_eq!(tags(&sandbox, &row), first);
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    assert!(!stdout(&output).contains("private-topic"));
    assert!(
        !fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl"))
            .unwrap()
            .contains("private-topic")
    );
}

#[test]
fn rejecting_a_managed_tag_keeps_operator_tags_and_global_vocabulary() {
    let (sandbox, row, path) = configured_tags();
    success(&confirm(
        &sandbox,
        row.id.as_str(),
        "--tag",
        "Private topic",
    ));
    let note = saved(&sandbox, &row);
    let edited = note.text.replace(
        "tags:\n  - \"private-topic\"",
        "tags: [private-topic, operator]",
    );
    assert_ne!(edited, note.text);
    fs::write(&note.target, edited).unwrap();
    success(&confirm(
        &sandbox,
        row.id.as_str(),
        "--reject-tag",
        "PRIVATE/TOPIC",
    ));
    let rejected = saved(&sandbox, &row);
    assert!(rejected.managed_tags.is_empty());
    assert_eq!(
        vpt_adapters::note_document::Document::parse(&rejected.text)
            .unwrap()
            .tags,
        ["operator"]
    );
    assert_eq!(tags(&sandbox, &row)[0].state, State::Rejected);
    assert_eq!(fs::read_to_string(&path).unwrap(), "private-topic\n");
    assert_eq!(rejected.content_digest, note.content_digest);
}
