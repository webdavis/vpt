use super::*;

pub(super) fn alert_hook(sandbox: &Sandbox, threshold: u32) {
    let script = sandbox.path().join("bin/alert");
    fs::write(&script, "#!/bin/sh\ncapture=\"$1\"\ndirectory=\"$2\"\ncount=\"$3\"\n/bin/cat >> \"$capture\"\nprintf '\\n' >> \"$capture\"\nset -- \"$directory\"/*.md\nprintf '%s\\n' \"$#\" >> \"$count\"\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(sandbox.config_path(), format!("{config}\n[alert]\naggregate_after={threshold}\ncommand=[{script:?}, {:?}, {:?}, {:?}]\n", sandbox.path().join("alerts.jsonl"), sandbox.path().join("home/.vpt/transcripts"), sandbox.path().join("notes-at-alert"))).unwrap();
}
pub(super) fn alerts(sandbox: &Sandbox) -> Vec<serde_json::Value> {
    fs::read_to_string(sandbox.path().join("alerts.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn multi_record_review_alert_is_aggregated_after_all_notes_are_published() {
    let sandbox = configured();
    for index in 0..4 {
        add(&sandbox, &format!("{index}.m4a"), 1_787_604_456 + index);
    }
    alert_hook(&sandbox, 3);
    json(&invoke(&sandbox, &[]));
    let events = alerts(&sandbox);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "review_needed");
    assert_eq!(events[0]["counts"]["recordings"], 4);
    assert_eq!(events[0]["counts"]["diagnostics"], 4);
    assert!(events[0]["recording"].is_null());
    assert_eq!(events[0]["paths"], serde_json::json!({}));
    assert_eq!(
        fs::read_to_string(sandbox.path().join("notes-at-alert")).unwrap(),
        "4\n"
    );
    assert!(!events[0].to_string().contains("Private words"));
    json(&invoke(&sandbox, &[]));
    assert_eq!(alerts(&sandbox).len(), 1);
}

#[test]
fn a_later_engine_failure_still_reports_successfully_published_notes_for_review() {
    let sandbox = configured();
    for index in 0..3 {
        add(&sandbox, &format!("{index}.m4a"), 1_787_604_456 + index);
    }
    alert_hook(&sandbox, 3);
    fs::write(sandbox.path().join("bin/engine"), "#!/bin/sh\nprintf 'called\\n' >> \"$1\"\ncount=$(/usr/bin/wc -l < \"$1\")\nif [ \"$count\" -eq 3 ]; then exit 17; fi\n/bin/cat -- \"$2\"\n").unwrap();
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["completed"].as_array().unwrap().len(), 3);
    let events = alerts(&sandbox);
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "review_needed")
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "transcribe_failed")
            .count(),
        1
    );
    assert_eq!(
        ledger(&sandbox)
            .recordings()
            .unwrap()
            .iter()
            .filter(|row| row.stages.note == StageState::Succeeded)
            .count(),
        2
    );
}

#[test]
fn disabled_incomplete_stages_are_reenabled_without_repeating_prior_transcription() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    sql(&sandbox, "UPDATE recordings SET stage_note='disabled'");
    let result = json(&invoke(&sandbox, &[]));
    assert_eq!(result["transcribed"], serde_json::json!([]));
    assert_eq!(result["notes"].as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    sql(
        &sandbox,
        "UPDATE recordings SET stage_transcribe='disabled'",
    );
    assert_eq!(
        json(&invoke(&sandbox, &[]))["transcribed"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn note_only_publication_failure_reports_the_committed_recording() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    sql(
        &sandbox,
        "UPDATE recordings SET stage_note='pending'; CREATE TRIGGER refuse_clear BEFORE DELETE ON dirty_publications BEGIN SELECT RAISE(ABORT,'Private canary'); END;",
    );
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private canary"));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([row.id.as_str()])
    );
    assert_eq!(
        ledger(&sandbox)
            .by_id(&row.id)
            .unwrap()
            .unwrap()
            .stages
            .note,
        StageState::Pending
    );
}

#[test]
fn readability_change_refreshes_successful_notes_without_rerunning_engines() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let original = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let config = fs::read_to_string(sandbox.config_path()).unwrap();
    fs::write(
        sandbox.config_path(),
        format!("{config}\n[readability]\nenabled = true\n"),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let plan = json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(plan["plan"][0]["stage"], "note");
    assert_eq!(fixture::snapshot(&sandbox), before);
    let result = json(&invoke(&sandbox, &[]));
    assert_eq!(result["transcribed"], serde_json::json!([]));
    assert_eq!(result["notes"].as_array().unwrap().len(), 1);
    let after = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert!(after.text.contains("[00:00] Private words."));
    assert_ne!(original.content_digest, after.content_digest);
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn a_named_note_refusal_keeps_its_exit_status_and_operator_bytes() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    json(&invoke(&sandbox, &[]));
    let row = ledger(&sandbox).recordings().unwrap().remove(0);
    let note = ledger(&sandbox)
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let changed = note.text.replace("<!-- vpt:content end -->", "");
    fs::write(&note.target, &changed).unwrap();
    let output = invoke(&sandbox, &[]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["rule"], "markers");
    assert_eq!(fs::read_to_string(&note.target).unwrap(), changed);
    assert_eq!(
        ledger(&sandbox)
            .note(&row.id, NoteStage::Transcript)
            .unwrap(),
        Some(note)
    );
}
