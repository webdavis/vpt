use super::*;

#[test]
fn missing_artifacts_list_only_stages_with_existing_files() {
    let (sandbox, id) = configured("handoff-available-stages");
    allow(&sandbox);
    let release = json(&invoke(&sandbox, &["redact", id.as_str(), "--json"]));
    fs::remove_file(release["written"].as_str().unwrap()).unwrap();
    let note = ledger(&sandbox)
        .note(&id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    let before = fixture::snapshot(&sandbox);
    let missing = handoff(&sandbox, id.as_str(), "analysis");
    assert_eq!(missing.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&missing.stderr).unwrap();
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .ends_with("available stages: transcript")
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    fs::remove_file(&note.target).unwrap();
    let before = fixture::snapshot(&sandbox);
    for stage in ["transcript", "released"] {
        let missing = handoff(&sandbox, id.as_str(), stage);
        assert_eq!(missing.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_slice(&missing.stderr).unwrap();
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap()
                .ends_with("available stages: none")
        );
    }
    assert_eq!(fixture::snapshot(&sandbox), before);
}
