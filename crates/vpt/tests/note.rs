#![cfg(feature = "dev-tools")]
#![allow(dead_code, reason = "shared CLI fixtures")]
#[path = "note/analysis.rs"]
mod analysis;
#[path = "show_list_storage/fixture.rs"]
mod fixture;
#[path = "note/mentions.rs"]
mod mentions;
#[path = "note/output.rs"]
mod output;
#[path = "note/path.rs"]
mod path;
#[path = "note/relation_confirm.rs"]
mod relation_confirm;
#[path = "note/relations.rs"]
mod relations;
#[path = "note/rename.rs"]
mod rename;
mod support;
#[path = "note/tag_confirm.rs"]
mod tag_confirm;
#[path = "note/tags.rs"]
mod tags;
use support::{Sandbox, stderr, stdout};
use vpt_adapters::{RootDir, SqliteLedger};
use vpt_application::ports::*;
use vpt_domain::{
    run::{HistoryFact, RunKind, WorkOutcome},
    time::UtcInstant,
    transcript::Word,
};

#[path = "note/setup.rs"]
mod setup;
use setup::*;

#[test]
fn writes_a_portable_note_from_saved_words_without_audio_or_an_engine() {
    let (sandbox, row) = configured();
    assert!(!row.audio_path.exists());
    let output = invoke(&sandbox, row.id.as_str());
    let result = success(&output);
    let note = saved(&sandbox, &row);
    assert_eq!(std::fs::read_to_string(&note.target).unwrap(), note.text);
    assert!(note.text.contains("[00:00] um private words"));
    assert!(note.text.contains("fixture:test"));
    assert!(note.text.contains("comparison:second"));
    assert!(!note.text.contains("\nstatus:"));
    assert_eq!(result["record"]["id"], row.id.as_str());
    assert!(!stdout(&output).contains("private words"));
    assert_eq!(
        note.target.file_name().unwrap(),
        "2026-08-24-title-7-07070707.md"
    );
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert!(ledger.pending_publications().unwrap().is_empty());
    assert_eq!(
        ledger.by_id(&row.id).unwrap().unwrap().stages.note,
        StageState::Succeeded
    );
}

#[test]
fn rewriting_keeps_the_pin_and_operator_content_while_applying_readability() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    let edited = first.text.replace(
        "tags: []",
        "tags: [manual]\nstatus: complete\ncustom: 'operator field'",
    ) + "\nOperator ending\n";
    std::fs::write(&first.target, &edited).unwrap();
    fixture::write_config(
        &sandbox,
        "[readability]\nenabled = true\n[note]\nslug_max_chars = 1\n",
    );
    success(&invoke(&sandbox, row.id.as_str()));
    let next = saved(&sandbox, &row);
    assert_eq!(next.target, first.target);
    assert_eq!(next.first_written, first.first_written);
    assert!(
        next.text
            .contains("status: complete\ncustom: 'operator field'")
    );
    assert!(next.text.ends_with("\nOperator ending\n"));
    assert!(next.text.contains("[00:00] Private words."));
    assert!(next.text.contains("manual"));
    assert_ne!(next.content_digest, first.content_digest);
}

#[test]
fn malformed_owned_note_is_preserved_and_refused() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    let edited = first.text.replace("<!-- vpt:content end -->", "");
    std::fs::write(&first.target, &edited).unwrap();
    let output = invoke(&sandbox, row.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read_to_string(&first.target).unwrap(), edited);
    assert_eq!(saved(&sandbox, &row), first);
}

#[test]
fn an_untracked_occupant_is_preserved_even_with_matching_identity() {
    let (sandbox, row) = configured();
    let directory = sandbox.path().join("home/.vpt/transcripts");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("2026-08-24-title-7-07070707.md");
    let foreign = format!("---\nvptRecording: {}\n---\nOperator file", row.id);
    std::fs::write(&path, &foreign).unwrap();
    let output = invoke(&sandbox, row.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(std::fs::read_to_string(path).unwrap(), foreign);
}

#[test]
fn obsidian_uses_configured_links_and_preserves_operator_frontmatter() {
    let (sandbox, row) = configured();
    let vault = sandbox.path().join("vault");
    std::fs::create_dir(&vault).unwrap();
    fixture::write_config(
        &sandbox,
        &format!(
            "[note]\nprofile = 'obsidian'\nlink_style = 'markdown'\n[note.obsidian]\nvault_root = {vault:?}\nhub_transcripts = 'Voice notes'\n"
        ),
    );
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    assert!(first.text.contains(&format!("../audio/{}.m4a", row.id)));
    assert!(first.text.contains("Voice%20notes.md"));
    assert!(first.text.contains("status: \"active\""));
    assert!(!first.text.contains("[["));
    let edited = first
        .text
        .replace("status: \"active\"", "status: archived")
        .replace(
            "description: \"Voice memo recorded 2026-08-24\"",
            "description: 'mine'",
        );
    std::fs::write(&first.target, edited).unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    let second = saved(&sandbox, &row);
    assert!(second.text.contains("status: archived"));
    assert!(second.text.contains("description: 'mine'"));
    assert_eq!(first.first_written, second.first_written);
    assert_eq!(first.write_offset, second.write_offset);
}

#[test]
fn unknown_recording_and_missing_transcript_do_not_start_mutating_work() {
    let sandbox = fixture::configured("note-missing-recording");
    let row = fixture::record(&sandbox, 7, StageStates::fresh());
    let before = fixture::snapshot(&sandbox);
    assert_eq!(invoke(&sandbox, row.id.as_str()).status.code(), Some(2));
    assert_eq!(fixture::snapshot(&sandbox), before);
    fixture::seed(&sandbox, vec![row.clone()]);
    let before = fixture::snapshot(&sandbox);
    assert_eq!(invoke(&sandbox, row.id.as_str()).status.code(), Some(2));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn a_future_schema_is_readable_but_not_rewritten() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    let text = first.text.replace("vptSchema: 1", "vptSchema: 2");
    std::fs::write(&first.target, &text).unwrap();
    let output = invoke(&sandbox, row.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["rule"], "schema_ahead");
    assert_eq!(std::fs::read_to_string(&first.target).unwrap(), text);
    assert_eq!(saved(&sandbox, &row), first);
}

#[test]
fn explicit_regeneration_preserves_saved_operator_edits_after_expiration() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let first = saved(&sandbox, &row);
    let edited = first
        .text
        .replace("tags: []", "tags: [manual]\ncustom: 'keep me'")
        + "\nOperator ending\n";
    std::fs::write(&first.target, edited).unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    let before = saved(&sandbox, &row);
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection
        .execute(
            "UPDATE recordings SET stage_note = 'expired' WHERE id = ?1",
            [row.id.as_str()],
        )
        .unwrap();
    drop(connection);
    std::fs::remove_file(&before.target).unwrap();
    fixture::write_config(&sandbox, "[readability]\nenabled = true\n");
    success(&invoke(&sandbox, row.id.as_str()));
    let after = saved(&sandbox, &row);
    assert!(after.text.contains("custom: 'keep me'"));
    assert!(after.text.contains("manual"));
    assert!(after.text.ends_with("\nOperator ending\n"));
    assert!(after.text.contains("[00:00] Private words."));
    assert_eq!(after.target, before.target);
    assert_eq!(after.first_written, before.first_written);
    assert_eq!(after.write_offset, before.write_offset);
}
