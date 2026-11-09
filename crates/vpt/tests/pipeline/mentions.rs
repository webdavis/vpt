use super::*;
use std::io::Write;

#[test]
fn changed_index_refreshes_mentions_without_engines_and_dry_run_is_observational() {
    let sandbox = configured();
    add(&sandbox, "one.m4a", 1_787_604_456);
    let root = sandbox.path().join("home/.vpt/briefs");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Private.md"), "External first").unwrap();
    let terms = sandbox.config_path().with_file_name("known-terms.txt");
    fs::write(&terms, "Private\n").unwrap();
    writeln!(
        fs::OpenOptions::new()
            .append(true)
            .open(sandbox.config_path())
            .unwrap(),
        "[reconcile]\nknown_terms_path={terms:?}"
    )
    .unwrap();
    json(&invoke(&sandbox, &[]));
    let ledger = ledger(&sandbox);
    let row = ledger.recordings().unwrap().remove(0);
    assert_eq!(ledger.relations(&row.id).unwrap().len(), 1);
    assert!(ledger.mention_ambiguities(&row.id).unwrap().is_empty());
    let first = ledger
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert!(first.text.contains("- Mentions: [Private]"));
    fs::write(
        root.join("Second.md"),
        "---\naliases: [Private]\n---\nExternal second",
    )
    .unwrap();
    fs::remove_file(sandbox.path().join("bin/engine")).unwrap();
    let before = fixture::snapshot(&sandbox);
    let planned = json(&invoke(&sandbox, &["--dry-run"]));
    assert_eq!(
        planned["plan"],
        serde_json::json!([{"id":row.id.as_str(), "stage":"note"}])
    );
    assert_eq!(fixture::snapshot(&sandbox), before);
    let changed = json(&invoke(&sandbox, &[]));
    assert_eq!(changed["transcribed"], serde_json::json!([]));
    assert_eq!(changed["notes"].as_array().unwrap().len(), 1);
    assert!(ledger.relations(&row.id).unwrap().is_empty());
    let ambiguities = ledger.mention_ambiguities(&row.id).unwrap();
    assert_eq!(ambiguities.len(), 1);
    assert_eq!(ambiguities[0].term, "Private");
    assert_eq!(ambiguities[0].ranges.len(), 1);
    assert_eq!(ambiguities[0].ranges[0], 0..1);
    let after = ledger
        .note(&row.id, NoteStage::Transcript)
        .unwrap()
        .unwrap();
    assert_eq!(after.content_digest, first.content_digest);
    assert!(!after.text.contains("- Mentions:"));
    assert_eq!(json(&invoke(&sandbox, &[]))["notes"], serde_json::json!([]));
    assert_eq!(
        fs::read_to_string(sandbox.path().join("calls")).unwrap(),
        "called\n"
    );
}
