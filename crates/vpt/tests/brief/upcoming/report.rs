use super::*;

#[test]
fn provider_label_selects_confirmed_evidence_and_report_hook_receives_only_safe_fields() {
    let sandbox = configured();
    let id = crate::evidence::seed(&sandbox);
    let capture = sandbox.path().join("report.jsonl");
    let hook = sandbox.path().join("bin/report");
    fs::write(
        &hook,
        format!(
            "#!/bin/sh\n/bin/cat >> '{}'\nprintf '\\n' >> '{}'\n",
            capture.display(),
            capture.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    let config =
        fs::read_to_string(sandbox.config_path()).unwrap() + "\n[report]\ncommand=['report']\n";
    fs::write(sandbox.config_path(), config).unwrap();
    let mut row = event("one", SystemClock.now().secs + 3600);
    row["labels"] = json!(["planning"]);
    reply(&sandbox, vec![row]);
    let result = json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]));
    let occasion =
        vpt_domain::occasion::OccasionId::parse(result["written"][0].as_str().unwrap()).unwrap();
    let ledger = SqliteLedger::open_read_only(&sandbox.path().join("state/vpt"))
        .unwrap()
        .unwrap();
    let brief = ledger.brief(&occasion).unwrap().unwrap();
    assert_eq!(brief.snapshot.pack.items.len(), 1);
    assert_eq!(brief.snapshot.pack.items[0].source.recording, id);
    assert_eq!(
        brief.snapshot.pack.items[0].selector,
        vpt_domain::brief::Selector::Tag
    );
    let text = fs::read_to_string(&capture).unwrap();
    assert_eq!(text.lines().count(), 1);
    let event: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(event["schema"], "vpt.event/1");
    assert_eq!(event["channel"], "report");
    assert_eq!(event["event"], "brief_written");
    assert_eq!(event["occasion"], occasion.as_str());
    assert_eq!(event["counts"]["notes"], 1);
    assert_eq!(event["counts"]["spans"], 1);
    assert!(!text.contains("Private accepted quote"));
    assert!(!text.contains("planning"));
    json(&invoke(&sandbox, &["brief", "--upcoming", "--json"]));
    assert_eq!(fs::read_to_string(capture).unwrap(), text);
}
