use super::*;
#[test]
fn redact_omits_or_marks_open_spans_and_freezes_the_original_review_snapshot() {
    let sandbox = fixture::configured("redact-flags");
    let id = recording::seed_flags(&sandbox, true);
    let first = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    assert_eq!(first["omitted"], 1);
    assert_eq!(first["marked"], 0);
    assert!(first["masks"].get("number").is_none());
    let ledger = ledger(&sandbox);
    let old = ledger
        .release(first["sequence"].as_i64().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(old.provenance.open_lexical_flags, 1);
    fixture::write_config(&sandbox, "[share.redact]\nflagged_spans='mark'\n");
    let second = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    assert_eq!(second["marked"], 1);
    assert_eq!(second["omitted"], 0);
    assert_eq!(second["masks"]["number"], 1);
    assert!(
        fs::read_to_string(second["written"].as_str().unwrap())
            .unwrap()
            .contains("unverified")
    );
    assert!(second["collision"].as_bool().unwrap());
    let flag = ledger
        .transcript_flags(&id)
        .unwrap()
        .into_iter()
        .find(|f| matches!(f.content, vpt_domain::review::FlagContent::Lexical(_)))
        .unwrap();
    json(&invoke(
        &sandbox,
        &[
            "review",
            id.as_str(),
            "--resolve",
            &flag.identifier.hex(),
            "--confirm",
            "--json",
        ],
    ));
    assert_eq!(ledger.release(old.sequence).unwrap().unwrap(), old);
    let third = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    let latest = ledger
        .release(third["sequence"].as_i64().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(latest.provenance.open_lexical_flags, 0);
    assert_eq!(latest.provenance.open_flags, 0);
    assert!(!latest.provenance.reviewed); // The single-engine diagnostic remains open.
}
#[test]
fn redact_keeps_title_and_date_minimal_and_reports_collisions_in_human_output() {
    let (sandbox, id) = configured("redact-header");
    fixture::write_config(&sandbox, "[share]\ninclude_date=true\nsource_line=''\n");
    let first = json(&invoke(
        &sandbox,
        &["redact", id.as_str(), "--title", "Public", "--json"],
    ));
    let text = fs::read_to_string(first["written"].as_str().unwrap()).unwrap();
    assert!(text.starts_with(&format!(
        "---\ntitle: \"Public\"\ndate: \"{}\"\n---\n",
        id.capture_date()
    )));
    let second = invoke(&sandbox, &["redact", id.as_str(), "--title", "Different"]);
    assert_eq!(second.status.code(), Some(0), "{}", stderr(&second));
    assert!(String::from_utf8_lossy(&second.stdout).contains("Filename collision"));
    assert!(!String::from_utf8_lossy(&second.stdout).contains("Alice"));
}
#[test]
fn redact_completion_failure_is_repaired_by_next_mutating_command_from_saved_bytes() {
    let (sandbox, id) = configured("redact-recovery");
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    connection.execute_batch("CREATE TRIGGER refuse_release_completion BEFORE INSERT ON release_completions BEGIN SELECT RAISE(ABORT,'Private failure'); END").unwrap();
    fixture::preserve_wal(&connection);
    let result = invoke(&sandbox, &["redact", id.as_str(), "--json"]);
    assert_eq!(result.status.code(), Some(1), "{}", stderr(&result));
    assert!(!stderr(&result).contains("Private"));
    let ledger = ledger(&sandbox);
    assert!(ledger.releases(&id).unwrap().is_empty());
    let sequence = ledger.pending_releases().unwrap()[0];
    let payload = ledger.release_payload(sequence).unwrap().unwrap();
    assert_eq!(
        fs::read_to_string(&payload.release.destination).unwrap(),
        payload.text
    );
    assert_eq!(
        fs::read_to_string(&payload.release.report_path).unwrap(),
        payload.report
    );
    connection
        .execute_batch("DROP TRIGGER refuse_release_completion")
        .unwrap();
    drop(connection);
    json(&invoke(&sandbox, &["note", "write", id.as_str(), "--json"]));
    assert_eq!(ledger.releases(&id).unwrap(), [sequence]);
    assert!(ledger.pending_releases().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(&payload.release.destination).unwrap(),
        payload.text
    );
    assert_eq!(
        ledger.release(sequence).unwrap().unwrap().provenance,
        payload.release.provenance
    );
}
#[test]
fn redact_broken_stdout_preserves_completed_recording_and_selectable_release() {
    let (sandbox, id) = configured("redact-output-failure");
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args(["redact", id.as_str(), "--json"]),
        std::time::Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error["error"]["completed"],
        serde_json::json!([id.as_str()])
    );
    assert_eq!(ledger(&sandbox).releases(&id).unwrap().len(), 1);
}
#[test]
fn redact_refuses_changed_managed_source_but_discards_operator_prose() {
    let (sandbox, id) = configured("redact-source");
    let ledger = ledger(&sandbox);
    let note = ledger.note(&id, NoteStage::Transcript).unwrap().unwrap();
    let text = format!("{}\nPrivate operator prose\n", note.text);
    fs::write(&note.target, &text).unwrap();
    let first = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    assert!(
        !fs::read_to_string(first["written"].as_str().unwrap())
            .unwrap()
            .contains("operator")
    );
    assert_eq!(fs::read_to_string(&note.target).unwrap(), text);
    fs::write(&note.target, text.replace("Meet", "Changed")).unwrap();
    let failed = invoke(&sandbox, &["redact", id.as_str(), "--json"]);
    assert_eq!(failed.status.code(), Some(3), "{}", stderr(&failed));
    assert!(stderr(&failed).contains("redact_source"));
    assert_eq!(ledger.releases(&id).unwrap().len(), 1);
}

#[test]
fn redact_reports_missing_source_as_input_error_and_read_failure_as_operational_error() {
    let (sandbox, id) = configured("redact-source-errors");
    let note = ledger(&sandbox)
        .note(&id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    fs::set_permissions(&note.target, fs::Permissions::from_mode(0o000)).unwrap();
    let unreadable = invoke(&sandbox, &["redact", id.as_str(), "--json"]);
    fs::set_permissions(&note.target, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(unreadable.status.code(), Some(1), "{}", stderr(&unreadable));
    fs::remove_dir_all(note.target.parent().unwrap()).unwrap();
    let absent = invoke(&sandbox, &["redact", id.as_str(), "--json"]);
    assert_eq!(absent.status.code(), Some(2), "{}", stderr(&absent));
    assert!(ledger(&sandbox).pending_releases().unwrap().is_empty());
}
