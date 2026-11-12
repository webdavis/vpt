use super::*;

#[test]
fn unknown_recording_and_absent_transcript_do_not_create_history() {
    let sandbox = fixture::configured("verify-absent");
    let row = fixture::record(&sandbox, 7, StageStates::fresh());
    let path = sandbox.path().join("external.md");
    fs::write(&path, "- claim").unwrap();
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        invoke(&sandbox, &path, row.id.as_str()).status.code(),
        Some(2)
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    fixture::seed(&sandbox, vec![row.clone()]);
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        invoke(&sandbox, &path, row.id.as_str()).status.code(),
        Some(2)
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn unregistered_copies_broken_markers_and_symlinks_remain_report_only() {
    let (sandbox, record) = note::configured();
    success(&note::invoke(&sandbox, record.id.as_str()));
    let owned = note::saved(&sandbox, &record);
    let external = sandbox.path().join("copy.md");
    fs::write(&external, &owned.text).unwrap();
    let link = sandbox.path().join("link.md");
    std::os::unix::fs::symlink(&owned.target, &link).unwrap();
    for path in [&external, &link] {
        let before = fixture::snapshot(&sandbox);
        assert_eq!(
            success(&invoke(&sandbox, path, record.id.as_str()))["owned"],
            false
        );
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
    fs::write(
        &owned.target,
        owned.text.replace("<!-- vpt:content end -->", ""),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        success(&invoke(&sandbox, &owned.target, record.id.as_str()))["owned"],
        false
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn foreign_recording_header_is_refused_before_any_mutation() {
    let (sandbox, record) = note::configured();
    let path = sandbox.path().join("foreign.md");
    fs::write(
        &path,
        format!(
            "---\nvpt-recording: {}\n---\n- Private claim\n",
            fixture::record(&sandbox, 9, StageStates::fresh()).id
        ),
    )
    .unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, &path, record.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["rule"], "ownership");
    assert!(!stderr(&output).contains("Private"));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn missing_nonregular_and_invalid_utf8_inputs_fail_without_mutation() {
    let (sandbox, record) = note::configured();
    let invalid = sandbox.path().join("invalid.md");
    fs::write(&invalid, [0xff]).unwrap();
    for path in [
        sandbox.path().join("missing.md"),
        sandbox.path().to_path_buf(),
        invalid,
    ] {
        let before = fixture::snapshot(&sandbox);
        let output = invoke(&sandbox, &path, record.id.as_str());
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn a_renamed_note_is_report_only_until_its_new_pin_is_committed() {
    let (sandbox, record) = note::configured();
    success(&note::invoke(&sandbox, record.id.as_str()));
    let owned = note::saved(&sandbox, &record);
    let renamed = owned.target.with_file_name("renamed.md");
    fs::rename(&owned.target, &renamed).unwrap();
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        success(&invoke(&sandbox, &renamed, record.id.as_str()))["owned"],
        false
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    success(&note::invoke(&sandbox, record.id.as_str()));
    assert_eq!(
        success(&invoke(&sandbox, &renamed, record.id.as_str()))["owned"],
        true
    );
}

#[test]
fn a_ledger_pin_outside_the_configured_store_does_not_authorize_writes() {
    let (sandbox, record) = note::configured();
    success(&note::invoke(&sandbox, record.id.as_str()));
    let owned = note::saved(&sandbox, &record);
    let outside = sandbox.path().join("outside.md");
    fs::rename(&owned.target, &outside).unwrap();
    let connection = rusqlite::Connection::open(sandbox.path().join("state/vpt/vpt.db")).unwrap();
    fixture::preserve_wal(&connection);
    connection
        .execute(
            "UPDATE owned_notes SET target=?1",
            [outside.to_str().unwrap()],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE recordings SET transcript_note_path=?1",
            [outside.to_str().unwrap()],
        )
        .unwrap();
    drop(connection);
    let before = fixture::snapshot(&sandbox);
    assert_eq!(
        success(&invoke(&sandbox, &outside, record.id.as_str()))["owned"],
        false
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
}
