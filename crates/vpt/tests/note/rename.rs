use super::*;

#[test]
fn unique_renamed_note_is_repinned_preserves_edits_and_logs_once() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let original = saved(&sandbox, &row);
    let renamed = original.target.with_file_name("Operator choice.md");
    std::fs::rename(&original.target, &renamed).unwrap();
    std::fs::write(&renamed, original.text.clone() + "\nOperator addition\n").unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    let after = saved(&sandbox, &row);
    assert_eq!(after.target, renamed);
    assert_eq!(after.first_written, original.first_written);
    assert!(after.text.ends_with("\nOperator addition\n"));
    assert!(!original.target.exists());
    assert_eq!(std::fs::read_to_string(&renamed).unwrap(), after.text);
    success(&invoke(&sandbox, row.id.as_str()));
    let events = std::fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    let moves: Vec<serde_json::Value> = events
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|line: &serde_json::Value| line["fact"]["tag"] == "note_moved")
        .collect();
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0]["fact"]["from"], original.target.to_str().unwrap());
    assert_eq!(moves[0]["fact"]["to"], renamed.to_str().unwrap());
    assert!(!events.contains("Operator addition"));
}

#[test]
fn missing_unexpired_note_is_regenerated_only_after_no_claimant_is_found() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let original = saved(&sandbox, &row);
    std::fs::remove_file(&original.target).unwrap();
    let unrelated = original.target.with_file_name("external.md");
    std::fs::write(&unrelated, "# External note\nNo frontmatter\n").unwrap();
    success(&invoke(&sandbox, row.id.as_str()));
    let after = saved(&sandbox, &row);
    assert_eq!(after.target, original.target);
    assert_eq!(after.first_written, original.first_written);
    assert_eq!(after.content_digest, original.content_digest);
    assert_eq!(
        std::fs::read_to_string(&original.target).unwrap(),
        after.text
    );
    assert_eq!(
        std::fs::read_to_string(unrelated).unwrap(),
        "# External note\nNo frontmatter\n"
    );
}

#[test]
fn ambiguous_renames_refuse_and_name_all_claimants_without_repinning() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let original = saved(&sandbox, &row);
    let first = original.target.with_file_name("First.md");
    let second = original.target.with_file_name("Second.md");
    std::fs::rename(&original.target, &first).unwrap();
    std::fs::copy(&first, &second).unwrap();
    let output = invoke(&sandbox, row.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["rule"], "ambiguous_note");
    assert!(stderr(&output).contains(first.to_str().unwrap()));
    assert!(stderr(&output).contains(second.to_str().unwrap()));
    assert_eq!(saved(&sandbox, &row), original);
    assert_eq!(std::fs::read_to_string(&first).unwrap(), original.text);
    assert_eq!(std::fs::read_to_string(&second).unwrap(), original.text);
    assert!(!original.target.exists());
}

#[test]
fn matching_identity_with_bad_markers_is_refused_in_rename_scan() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let original = saved(&sandbox, &row);
    let renamed = original.target.with_file_name("Broken.md");
    let text = original.text.replace("<!-- vpt:content end -->", "");
    std::fs::remove_file(&original.target).unwrap();
    std::fs::write(&renamed, &text).unwrap();
    let output = invoke(&sandbox, row.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["rule"], "markers");
    assert_eq!(saved(&sandbox, &row), original);
    assert_eq!(std::fs::read_to_string(renamed).unwrap(), text);
}

#[test]
fn redirected_claimants_are_never_followed_or_repinned() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let original = saved(&sandbox, &row);
    let outside = sandbox.path().join("outside.md");
    std::fs::rename(&original.target, &outside).unwrap();
    let redirected = original.target.with_file_name("Linked.md");
    std::os::unix::fs::symlink(&outside, &redirected).unwrap();
    let output = invoke(&sandbox, row.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(saved(&sandbox, &row), original);
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), original.text);
    assert_eq!(std::fs::read_link(&redirected).unwrap(), outside);
    assert!(!original.target.exists());
}

#[test]
fn foreign_occupant_at_the_pin_refuses_even_when_a_renamed_claimant_exists() {
    let (sandbox, row) = configured();
    success(&invoke(&sandbox, row.id.as_str()));
    let original = saved(&sandbox, &row);
    let renamed = original.target.with_file_name("Renamed.md");
    std::fs::rename(&original.target, &renamed).unwrap();
    let other = fixture::record(&sandbox, 8, StageStates::fresh());
    let foreign = original.text.replace(row.id.as_str(), other.id.as_str());
    std::fs::write(&original.target, &foreign).unwrap();
    let output = invoke(&sandbox, row.id.as_str());
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert_eq!(saved(&sandbox, &row), original);
    assert_eq!(std::fs::read_to_string(&original.target).unwrap(), foreign);
    assert_eq!(std::fs::read_to_string(&renamed).unwrap(), original.text);
}
