use super::*;
use vpt_domain::redaction::Options;
fn content() -> Content {
    let mut r = Redactor::new(&["Alice"], &[], Options::default()).unwrap();
    let content = redact_content("Alice met Bob.\n", &mut r, &ContentOptions::default()).unwrap();
    document::assemble(content, &document::Header::default(), &r).unwrap()
}
#[test]
fn private_report_binds_both_digests_maps_masks_and_accounts_for_every_generated_line() {
    let content = content();
    let source = "Operator\nAlice met Bob.\nTail\n";
    let report = encode(
        &content,
        source,
        "Operator\n".len(),
        std::path::Path::new("/notes/source.md"),
    )
    .unwrap();
    let json: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(json["schema"], "vpt.redaction-report/1");
    let digest = crate::ledger::brief::validate::digest;
    assert_eq!(
        json["source_artifact_sha256"],
        digest(source.as_bytes()).hex()
    );
    assert_eq!(
        json["released_content_sha256"],
        digest(content.text.as_bytes()).hex()
    );
    assert_eq!(json["mappings"][0]["label"], "[term 1]");
    assert_eq!(json["mappings"][0]["value"], "Alice");
    let lines = json["lines"].as_array().unwrap();
    assert_eq!(lines.len(), content.text.lines().count());
    assert!(lines[..4].iter().all(|l| l["source_bytes"].is_null()));
    assert_eq!(
        lines[4]["source_bytes"],
        serde_json::json!({"start":9,"end":24})
    );
    assert!(!content.text.contains("Alice"));
}
#[test]
fn report_refuses_stale_out_of_range_duplicate_or_non_utf8_source_spans() {
    for case in 0..4 {
        let mut content = content();
        match case {
            0 => content.lines[0].source.end = 999,
            1 => content.lines.push(content.lines[0].clone()),
            2 => content.lines[0].line = 999,
            _ => content.lines[0].source = 1..4,
        }
        assert!(matches!(
            encode(
                &content,
                "Élodie met Bob.\n",
                0,
                std::path::Path::new("/notes/source.md")
            ),
            Err(Error::InvalidText)
        ));
    }
}
