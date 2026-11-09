use super::*;
use std::time::Duration;

#[test]
fn output_failure_reports_owned_note_commit_and_keeps_external_verification_observational() {
    for owned in [true, false] {
        let (sandbox, record) = note::configured();
        let path = if owned {
            success(&note::invoke(&sandbox, record.id.as_str()));
            let saved = note::saved(&sandbox, &record);
            let document = vpt_adapters::note_document::Document::parse(&saved.text).unwrap();
            fs::write(
                &saved.target,
                document.replace_content("- Private claim\n").unwrap(),
            )
            .unwrap();
            saved.target
        } else {
            let path = sandbox.path().join("external.md");
            fs::write(&path, "- Private claim\n").unwrap();
            path
        };
        let before = fixture::snapshot(&sandbox);
        let output = support::run_with_broken_stdout(
            sandbox.vpt().arg("verify-note").arg(&path).args([
                "--recording",
                record.id.as_str(),
                "--json",
            ]),
            Duration::from_secs(5),
        );
        assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
        assert!(output.stdout.is_empty());
        assert!(!stderr(&output).contains("Private claim"));
        let document: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
        assert_eq!(document["error"]["message"], "standard output write failed");
        if owned {
            assert_eq!(
                document["error"]["completed"],
                serde_json::json!([record.id.as_str()])
            );
            let saved = note::saved(&sandbox, &record);
            assert!(saved.text.contains("[unsourced]"));
            assert_eq!(fs::read_to_string(path).unwrap(), saved.text);
        } else {
            assert_eq!(document["error"]["completed"], serde_json::json!([]));
            assert_eq!(fixture::snapshot(&sandbox), before);
        }
    }
}
