use crate::fixture::{configured, run_json};
use crate::support::{Sandbox, run, stderr, stdout};
use serde_json::Value;

fn glyph(result: &Value) -> &'static str {
    match result.as_str().unwrap() {
        "ok" => "✓",
        "warning" => "⚠",
        "issue" => "✗",
        _ => "·",
    }
}

fn human(sandbox: &Sandbox, code: i32) -> String {
    let output = run(sandbox.vpt().current_dir(sandbox.path()).arg("doctor"));
    assert_eq!(output.status.code(), Some(code), "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
    stdout(&output)
}

fn closing(report: &str) -> Vec<&str> {
    let (_, closing) = report.split_once("\n\nNeeds attention\n").unwrap();
    closing.lines().collect()
}

fn advice(value: &Value) -> Option<String> {
    value["advice"].as_str().map(|text| format!("    → {text}"))
}

#[test]
fn the_human_report_shows_the_machine_values_in_the_shared_style() {
    let sandbox = configured("doctor-human-values");
    std::fs::rename(
        sandbox.path().join("bin/vpt-macos"),
        sandbox.path().join("held-helper"),
    )
    .unwrap();
    let document = run_json(&sandbox, 1);
    let mut expected = vec![format!("vpt {}", env!("CARGO_PKG_VERSION"))];
    for check in document["checks"].as_array().unwrap() {
        expected.push(String::new());
        expected.push(format!(
            "{} ── {}",
            check["name"].as_str().unwrap(),
            check["detail"].as_str().unwrap()
        ));
        let rows = check["rows"].as_array().unwrap();
        if rows.is_empty() {
            expected.push("  · not run".into());
            expected.extend(advice(check));
        }
        for row in rows {
            expected.push(format!(
                "  {} {}: {}",
                glyph(&row["result"]),
                row["name"].as_str().unwrap(),
                row["detail"].as_str().unwrap()
            ));
            expected.extend(advice(row));
        }
    }
    expected.extend(
        [
            "",
            "Needs attention",
            "  ✗ 2 issues to fix, 3 warnings to look at",
        ]
        .map(String::from),
    );
    for (index, entry) in document["needs_attention"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        expected.push(format!(
            "  {}. {} {}: {}",
            index + 1,
            entry["check"].as_str().unwrap(),
            entry["name"].as_str().unwrap(),
            entry["detail"].as_str().unwrap()
        ));
    }
    let report = human(&sandbox, 1);
    assert_eq!(report, expected.join("\n") + "\n");
}

#[test]
fn warnings_alone_are_counted_and_withhold_the_all_clear() {
    let sandbox = configured("doctor-human-warnings");
    let report = human(&sandbox, 0);
    let closing = closing(&report);
    assert_eq!(closing[0], "  ⚠ 3 warnings to look at", "{report}");
    assert_eq!(closing.len(), 4, "{report}");
    assert!(closing[1].starts_with("  1. stores audio: "), "{report}");
}

#[test]
fn a_report_with_nothing_wrong_closes_by_saying_so() {
    let sandbox = configured("doctor-human-clear");
    std::fs::remove_dir(sandbox.path().join(".git")).unwrap();
    let report = human(&sandbox, 0);
    assert_eq!(
        closing(&report),
        ["  ✓ nothing needs attention"],
        "{report}"
    );
}
