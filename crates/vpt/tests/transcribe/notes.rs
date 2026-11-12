use super::*;
use std::fs;
use vpt_application::ports::{NoteLedger, NoteStage, ReviewLedger};

pub(super) fn target(
    sandbox: &Sandbox,
    row: &vpt_application::ports::RecordingRecord,
) -> std::path::PathBuf {
    sandbox.path().join("home/.vpt/transcripts").join(
        vpt_domain::note::filename::transcript_name(&row.id, row.title.as_deref(), 60).unwrap(),
    )
}

pub(super) fn hooks(sandbox: &Sandbox, path: &std::path::Path, fail: bool) {
    let script = sandbox.path().join("bin/alert");
    fs::write(&script, "#!/bin/sh\ntest -f \"$2\" || exit 41\n/usr/bin/grep -q 'vpt:content end' \"$2\" || exit 42\n/bin/cat >> \"$1\"\nprintf '\\n' >> \"$1\"\nprintf 'Private hook output'\nexit \"$3\"\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}\n[alert]\ncommand = [{script:?}, {:?}, {path:?}, '{}']\n[report]\ncommand = ['/usr/bin/touch', {:?}]\n", sandbox.path().join("alerts.jsonl"), if fail {"17"} else {"0"}, sandbox.path().join("report-called"))).unwrap();
}

pub(super) fn alerts(sandbox: &Sandbox) -> Vec<serde_json::Value> {
    fs::read_to_string(sandbox.path().join("alerts.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn transcription_publishes_note_before_private_safe_review_alert() {
    let (sandbox, row) = uncertainty::apple();
    let path = target(&sandbox, &row);
    hooks(&sandbox, &path, false);
    let result = json(&run(&sandbox, row.id.as_str(), &[]));
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .expect("transcription publishes a note");
    assert_eq!(note.target, path);
    assert!(note.text.contains("[unverified: SecretAlice]"));
    assert!(note.text.contains("vpt-open-flags: 2"));
    assert_eq!(fs::read_to_string(&path).unwrap(), note.text);
    assert_eq!(result["note"], path.to_str().unwrap());
    assert_eq!(result["flags"]["low_confidence"], 1);
    assert_eq!(result["flags"]["agreed_unverified"], 2);
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Succeeded
    );
    let events = alerts(&sandbox);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "review_needed");
    assert_eq!(events[0]["counts"]["agreed_unverified"], 2);
    assert_eq!(events[0]["counts"]["low_confidence"], 1);
    assert_eq!(events[0]["counts"]["diagnostics"], 1);
    assert_eq!(events[0]["paths"]["note"], path.to_str().unwrap());
    assert!(!events[0].to_string().contains("SecretAlice"));
    assert!(sandbox.path().join("report-called").exists());
    let log = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!log.contains("SecretAlice"));
    assert!(!log.contains("Private hook output"));
}

#[test]
fn malformed_note_keeps_new_accepted_words_pending_and_sends_no_alert() {
    let (sandbox, row) = uncertainty::apple();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .expect("note");
    let malformed = note.text.replace("<!-- vpt:content end -->", "");
    fs::write(&note.target, &malformed).unwrap();
    hooks(&sandbox, &note.target, false);
    let output = run(&sandbox, row.id.as_str(), &[]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    let saved = ledger(&sandbox).by_id(&row.id).unwrap().unwrap();
    assert_eq!(saved.stages.transcribe, StageState::Succeeded);
    assert_eq!(saved.stages.note, StageState::Pending);
    assert_eq!(fs::read_to_string(note.target).unwrap(), malformed);
    assert!(alerts(&sandbox).is_empty());
}

#[test]
fn failed_alert_preserves_transcription_and_note_success_with_warning() {
    let (sandbox, row) = uncertainty::apple();
    hooks(&sandbox, &target(&sandbox, &row), true);
    let result = json(&run(&sandbox, row.id.as_str(), &[]));
    assert!(
        result["warnings"]
            .as_array()
            .is_some_and(|warnings| !warnings.is_empty())
    );
    assert!(!result.to_string().contains("Private hook output"));
    assert_eq!(alerts(&sandbox).len(), 1);
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Succeeded
    );
    assert!(sandbox.path().join("report-called").exists());
}

#[test]
fn resolved_flags_stay_closed_and_corrections_survive_retranscription() {
    let (sandbox, row) = uncertainty::apple();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let snapshot = ledger(&sandbox).review_snapshot(&row.id).unwrap().unwrap();
    for flag in snapshot.flags {
        let action = if matches!(&flag.content, vpt_domain::review::FlagContent::Lexical(lexical) if lexical.class == vpt_domain::uncertainty::Class::LowConfidence)
        {
            vec!["--correct", "Private correction"]
        } else {
            vec!["--dismiss"]
        };
        let output = support::run_with_budget(
            sandbox
                .vpt()
                .args([
                    "review",
                    row.id.as_str(),
                    "--resolve",
                    &flag.identifier.hex(),
                    "--json",
                ])
                .args(action),
            std::time::Duration::from_secs(5),
        );
        json(&output);
    }
    hooks(&sandbox, &target(&sandbox, &row), false);
    json(&run(&sandbox, row.id.as_str(), &[]));
    assert!(alerts(&sandbox).is_empty());
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert!(note.text.contains("vpt-open-flags: 0"));
    assert!(note.text.contains("vpt-diagnostics: 0"));
    assert!(note.text.contains("[corrected: Private correction]"));
}

#[test]
fn retranscription_preserves_expired_notes_without_regeneration_or_review_alert() {
    let (sandbox, row) = uncertainty::apple();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection
        .execute("UPDATE recordings SET stage_note = 'expired'", [])
        .unwrap();
    drop(connection);
    fs::remove_file(&note.target).unwrap();
    hooks(&sandbox, &note.target, false);
    let result = json(&run(&sandbox, row.id.as_str(), &[]));
    assert!(result["note"].is_null());
    assert_eq!(result["flags"]["low_confidence"], 1);
    assert!(!note.target.exists());
    assert!(alerts(&sandbox).is_empty());
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Expired
    );
}

#[test]
fn transcription_uses_obsidian_readability_and_recovers_an_operator_renamed_note() {
    let (sandbox, row) = configured();
    let vault = sandbox.path().join("vault");
    fs::create_dir(&vault).unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}\n[readability]\nenabled = true\n[note]\nprofile = 'obsidian'\nlink_style = 'wiki'\n[note.obsidian]\nvault_root = {vault:?}\n")).unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let first = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert!(first.text.contains("vpt-readability: true"));
    assert!(first.text.contains("[00:00] Private transcript canary."));
    assert!(first.text.contains("status: \"active\""));
    assert!(first.text.contains(&format!(
        "# {}",
        first.target.file_stem().unwrap().to_str().unwrap()
    )));
    assert!(first.text.contains("<!-- vpt:links start -->"));
    let renamed = first.target.with_file_name("Operator choice.md");
    fs::rename(&first.target, &renamed).unwrap();
    fs::write(&renamed, first.text.clone() + "\nOperator addition\n").unwrap();
    json(&run(&sandbox, row.id.as_str(), &[]));
    let after = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert_eq!(after.target, renamed);
    assert_eq!(after.first_written, first.first_written);
    assert!(after.text.ends_with("\nOperator addition\n"));
    assert_eq!(
        ledger(&sandbox)
            .accepted_transcript(&row.id)
            .unwrap()
            .unwrap()
            .transcript
            .segments[0]
            .words[0]
            .text,
        "private transcript canary"
    );
}
