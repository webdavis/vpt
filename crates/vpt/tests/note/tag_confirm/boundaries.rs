use super::*;

#[test]
fn invalid_tags_and_unknown_recordings_are_observational() {
    let (sandbox, row, _) = configured_tags();
    let before = fixture::snapshot(&sandbox);
    for value in ["#private", "1234", "", "private!"] {
        let output = confirm(&sandbox, row.id.as_str(), "--tag", value);
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(!stderr(&output).contains("private"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
    let unknown = fixture::record(&sandbox, 99, StageStates::fresh());
    let output = confirm(&sandbox, unknown.id.as_str(), "--tag", "valid");
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn confirmation_cap_preserves_prior_tags_notes_and_vocabulary() {
    let (sandbox, row, path) = configured_tags();
    fixture::write_config(
        &sandbox,
        &format!("[tags]\nknown_tags_path={path:?}\nmax_per_note=1\n"),
    );
    success(&confirm(&sandbox, row.id.as_str(), "--tag", "first"));
    let note = saved(&sandbox, &row);
    let before = tags(&sandbox, &row);
    let output = confirm(&sandbox, row.id.as_str(), "--tag", "second");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(stderr(&output).contains("confirmed_tag_limit"));
    assert_eq!(saved(&sandbox, &row), note);
    assert_eq!(tags(&sandbox, &row), before);
    assert_eq!(fs::read_to_string(&path).unwrap(), "first\n");
}

#[test]
fn tags_can_be_confirmed_before_transcription_and_do_not_regenerate_expired_notes() {
    let (sandbox, row, _) = configured_tags();
    let untranscribed = fixture::record(&sandbox, 22, StageStates::fresh());
    SqliteLedger::open(&RootDir::open(&sandbox.path().join("state/vpt")).unwrap())
        .unwrap()
        .commit(&LedgerCommit {
            recordings: vec![untranscribed.clone()],
            ..Default::default()
        })
        .unwrap();
    success(&confirm(
        &sandbox,
        untranscribed.id.as_str(),
        "--tag",
        "early",
    ));
    assert_eq!(tags(&sandbox, &untranscribed)[0].name, "early");
    success(&invoke(&sandbox, row.id.as_str()));
    let note = saved(&sandbox, &row);
    fs::remove_file(&note.target).unwrap();
    super::durability::sql(&sandbox, "UPDATE recordings SET stage_note='expired'");
    success(&confirm(&sandbox, row.id.as_str(), "--tag", "later"));
    assert!(!note.target.exists());
    assert_eq!(saved(&sandbox, &row), note);
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    assert_eq!(
        ledger.by_id(&row.id).unwrap().unwrap().stages.note,
        StageState::Expired
    );
}
