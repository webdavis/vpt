use super::*;
use std::time::Duration;

#[test]
fn output_failure_keeps_correction_and_reports_its_recording_without_private_text() {
    let (sandbox, id, terms) = resolution::setup();
    let flag = resolution::identifier(&sandbox, &id, "numeric");
    let output = support::run_with_broken_stdout(
        sandbox.vpt().args([
            "review",
            id.as_str(),
            "--resolve",
            &flag,
            "--correct",
            "Private correction",
            "--json",
        ]),
        Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert!(!stderr(&output).contains("Private correction"));
    assert!(!stderr(&output).contains("SecretAlice"));
    let document: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(document["error"]["message"], "standard output write failed");
    assert_eq!(
        document["error"]["completed"],
        serde_json::json!([id.as_str()])
    );
    let inputs = resolution::inputs(&sandbox, &id);
    assert!(
        inputs.notes[0]
            .text
            .contains("[corrected: Private correction]")
    );
    assert_eq!(
        std::fs::read_to_string(&inputs.notes[0].target).unwrap(),
        inputs.notes[0].text
    );
    assert_eq!(
        std::fs::read_to_string(terms).unwrap(),
        "Private correction\n"
    );
}
