use super::resolution::*;
use super::*;
use std::fs;
use vpt_domain::review::{Resolution, ResolutionKind};

#[test]
fn correcting_a_renamed_note_preserves_operator_edits_and_invalidates_analysis() {
    let (sandbox, id, terms) = setup();
    json(&run(sandbox.vpt().args([
        "note",
        "write",
        id.as_str(),
        "--json",
    ])));
    let original = inputs(&sandbox, &id).notes.remove(0);
    let renamed = original.target.with_file_name("Operator choice.md");
    fs::rename(&original.target, &renamed).unwrap();
    fs::write(
        &renamed,
        original
            .text
            .clone()
            .replace("tags: []", "tags: [manual]\ncustom: 'keep'")
            + "\nOperator prose\n",
    )
    .unwrap();
    sql(
        &sandbox,
        "UPDATE recordings SET stage_analysis = 'succeeded'",
    );
    let flag = identifier(&sandbox, &id, "numeric");
    json(&resolve(
        &sandbox,
        &id,
        &flag,
        &["--correct", "Private\n[correction]"],
    ));
    let after = inputs(&sandbox, &id);
    assert_eq!(after.record.stages.analysis, StageState::Pending);
    let note = &after.notes[0];
    assert_eq!(note.target, renamed);
    assert_eq!(note.first_written, original.first_written);
    assert!(note.text.contains("custom: 'keep'"));
    assert!(note.text.contains("manual"));
    assert!(note.text.ends_with("\nOperator prose\n"));
    assert!(
        note.text
            .contains("[corrected: Private&#10;&#91;correction&#93;]")
    );
    assert_eq!(fs::read_to_string(terms).unwrap(), "Private [correction]\n");
    assert!(!original.target.exists());
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert_eq!(events.matches("\"tag\":\"note_moved\"").count(), 1);
}

fn sql(sandbox: &Sandbox, statement: &str) {
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection.execute_batch(statement).unwrap();
}

#[test]
fn a_late_transaction_failure_rolls_back_resolution_note_and_terms() {
    let (sandbox, id, terms) = setup();
    json(&run(sandbox.vpt().args([
        "note",
        "write",
        id.as_str(),
        "--json",
    ])));
    let before = inputs(&sandbox, &id);
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_resolution_history BEFORE INSERT ON history_records WHEN instr(NEW.fact, 'flag_resolved') > 0 BEGIN SELECT RAISE(ABORT, 'Private canary'); END;",
    );
    let flag = identifier(&sandbox, &id, "numeric");
    let output = resolve(&sandbox, &id, &flag, &["--correct", "Private correction"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert!(!stderr(&output).contains("Private"));
    let after = inputs(&sandbox, &id);
    assert_eq!(after.review.flags, before.review.flags);
    assert_eq!(after.notes, before.notes);
    assert_eq!(
        fs::read_to_string(&before.notes[0].target).unwrap(),
        before.notes[0].text
    );
    assert!(!terms.exists());
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    assert!(
        ledger(&sandbox)
            .vocabulary_publication(&terms)
            .unwrap()
            .is_none()
    );
}

#[test]
fn failed_publication_acknowledgement_preserves_committed_state_and_retry_repairs_it() {
    let (sandbox, id, terms) = setup();
    let flag = identifier(&sandbox, &id, "numeric");
    sql(
        &sandbox,
        "CREATE TRIGGER refuse_clear BEFORE DELETE ON dirty_publications BEGIN SELECT RAISE(ABORT, 'Private canary'); END;",
    );
    let output = resolve(&sandbox, &id, &flag, &["--correct", "Private correction"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(!stderr(&output).contains("Private"));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([id.as_str()])
    );
    let before = inputs(&sandbox, &id);
    assert_eq!(
        before
            .review
            .flags
            .iter()
            .find(|f| f.identifier.hex() == flag)
            .unwrap()
            .resolution
            .kind(),
        Some(ResolutionKind::Corrected)
    );
    assert!(!ledger(&sandbox).pending_publications().unwrap().is_empty());
    sql(&sandbox, "DROP TRIGGER refuse_clear");
    json(&resolve(
        &sandbox,
        &id,
        &flag,
        &["--correct", "Private correction"],
    ));
    let after = inputs(&sandbox, &id);
    assert_eq!(before.review.flags, after.review.flags);
    assert_eq!(fs::read_to_string(terms).unwrap(), "Private correction\n");
    assert_eq!(
        fs::read_to_string(&after.notes[0].target).unwrap(),
        after.notes[0].text
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert_eq!(events.matches("\"tag\":\"flag_resolved\"").count(), 1);
}

fn ledger(sandbox: &Sandbox) -> SqliteLedger {
    SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap()
}

#[test]
fn redirected_known_terms_prevent_resolution_and_leave_external_bytes_untouched() {
    let (sandbox, id, terms) = setup();
    let outside = sandbox.path().join("outside.txt");
    fs::write(&outside, "Operator words\n").unwrap();
    std::os::unix::fs::symlink(&outside, &terms).unwrap();
    let before = inputs(&sandbox, &id);
    let flag = identifier(&sandbox, &id, "agreed-unverified");
    let output = resolve(&sandbox, &id, &flag, &["--confirm"]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let after = inputs(&sandbox, &id);
    assert_eq!(after.review.flags, before.review.flags);
    assert!(after.notes.is_empty());
    assert_eq!(fs::read_to_string(outside).unwrap(), "Operator words\n");
}

#[test]
fn option_like_correction_is_literal_and_a_later_action_can_replace_it() {
    let (sandbox, id, terms) = setup();
    let flag = identifier(&sandbox, &id, "numeric");
    json(&resolve(&sandbox, &id, &flag, &["--correct", "--term"]));
    assert_eq!(fs::read_to_string(&terms).unwrap(), "--term\n");
    let output = run(sandbox
        .vpt()
        .args(["review", id.as_str(), "--resolve", &flag, "--dismiss"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("2 open lexical flags"));
    let after = inputs(&sandbox, &id);
    assert!(matches!(
        after
            .review
            .flags
            .iter()
            .find(|f| f.identifier.hex() == flag)
            .unwrap()
            .resolution,
        Resolution::Dismissed { .. }
    ));
    assert!(!after.notes[0].text.contains("[corrected:"));
    assert_eq!(fs::read_to_string(terms).unwrap(), "--term\n");
}

#[test]
fn log_export_failure_keeps_success_and_reports_a_safe_warning() {
    let (sandbox, id, _) = setup();
    fs::create_dir(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    let flag = identifier(&sandbox, &id, "low-confidence");
    let result = json(&resolve(&sandbox, &id, &flag, &["--dismiss"]));
    assert!(
        result["warnings"]
            .as_array()
            .is_some_and(|warnings| !warnings.is_empty())
    );
    assert_eq!(result["open_flags"], 2);
}
