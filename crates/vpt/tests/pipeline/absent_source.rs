use super::*;

#[test]
fn run_without_a_recordings_folder_still_transcribes_imported_recordings() {
    let sandbox = configured();
    fs::remove_dir(sandbox.path().join("voice-memos/Recordings")).unwrap();
    let file = sandbox.path().join("downloads/imported.m4a");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(
        &file,
        vpt_domain::fixtures::m4a(1_787_604_456, 12, b"import"),
    )
    .unwrap();
    let imported = json(&support::run_with_budget(
        sandbox.vpt().args(["import", "--json"]).arg(&file),
        std::time::Duration::from_secs(8),
    ));
    let id = imported["ingested"][0]["id"].clone();
    let result = json(&invoke(&sandbox, &[]));
    assert_eq!(result["ingested"], serde_json::json!([]));
    assert_eq!(result["transcribed"], serde_json::json!([id]));
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line
                .as_str()
                .unwrap()
                .starts_with("Voice Memos has not stored"))
    );
    assert!(!sandbox.path().join("voice-memos/Recordings").exists());
}
