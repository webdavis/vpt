use crate::fixture::Trial;
use crate::support::{stderr, stdout};

#[test]
fn an_unknown_part_is_a_usage_error_naming_the_valid_ones() {
    let trial = Trial::new("test-unknown-part");
    let output = trial.output(&["test", "engines", "made-up-part"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let text = stderr(&output);
    assert!(text.starts_with("vpt: invalid value for "), "{text}");
    assert!(
        text.lines()
            .next()
            .unwrap()
            .ends_with("expected one of engines, speakers"),
        "{text}"
    );
}

#[test]
fn a_part_named_twice_runs_once() {
    let trial = Trial::new("test-named-twice");
    let document = trial.json(&["test", "engines", "engines"], 0);
    assert_eq!(document["checks"].as_array().unwrap().len(), 1);
}

#[test]
fn the_human_report_follows_the_doctor_style_on_stdout() {
    let trial = Trial::new("test-human");
    let output = trial.output(&["test", "engines"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    let text = stdout(&output);
    assert!(
        text.starts_with(&format!("vpt {}\n\nengines ── ", env!("CARGO_PKG_VERSION"))),
        "{text}"
    );
    assert!(text.contains("\n  ✓ clip: "), "{text}");
    assert!(
        text.contains("\n  ✓ apple: heard 21 of 21 words in "),
        "{text}"
    );
    assert!(
        text.ends_with("\nNeeds attention\n  ✓ nothing needs attention\n"),
        "{text}"
    );
}

#[test]
fn an_issue_prints_its_advice_and_exits_one() {
    let trial = Trial::new("test-human-issue");
    std::fs::remove_file(trial.path().join("bin/say")).unwrap();
    let output = trial.output(&["test"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    let text = stdout(&output);
    assert!(text.contains("\n  ✗ clip: "), "{text}");
    assert!(text.contains("\n    → "), "{text}");
    assert!(text.contains("\n  1. engines clip: "), "{text}");
}

#[test]
fn a_missing_config_is_a_config_error() {
    let trial = Trial::new("test-no-config");
    std::fs::remove_file(trial.sandbox.config_path()).unwrap();
    let output = trial.output(&["test", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["kind"], "config");
}
