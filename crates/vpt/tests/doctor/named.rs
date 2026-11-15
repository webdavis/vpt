use crate::fixture::{CHECKS, check, configured, names, report, row};
use crate::support::{run, stderr, stdout};

const VALID: &str = "expected one of config, stores, ledger, recordings, helper, engines, speakers, summary, actions, leftovers";

#[test]
fn named_checks_run_alone_in_display_order_once_each() {
    let sandbox = configured("doctor-named");
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .args(["doctor", "engines", "helper", "engines", "--json"]));
    let document = report(&output, 0);
    assert_eq!(names(&document), ["helper", "engines"]);
    assert_eq!(row(&document, "engines", "apple")["result"], "ok");
}

#[test]
fn every_check_name_is_accepted_alone() {
    let sandbox = configured("doctor-each-name");
    for name in CHECKS {
        let output = run(sandbox
            .vpt()
            .current_dir(sandbox.path())
            .args(["doctor", name, "--json"]));
        assert_eq!(names(&report(&output, 0)), [name]);
    }
}

#[test]
fn a_named_check_without_a_config_reports_the_config_issue_and_exits_one() {
    let sandbox = configured("doctor-named-no-config");
    std::fs::rename(sandbox.config_path(), sandbox.path().join("held-config")).unwrap();
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .args(["doctor", "helper", "--json"]));
    let document = report(&output, 1);
    assert_eq!(names(&document), ["config", "helper"]);
    assert_eq!(row(&document, "config", "file")["result"], "issue");
    assert_eq!(check(&document, "helper")["result"], "not_run");
}

#[test]
fn an_unknown_check_name_is_a_usage_error_naming_the_valid_ones() {
    let sandbox = configured("doctor-unknown-name");
    let output =
        run(sandbox
            .vpt()
            .current_dir(sandbox.path())
            .args(["doctor", "helper", "made-up-check"]));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    let text = stderr(&output);
    assert!(text.starts_with("vpt: invalid value for "), "{text}");
    assert!(text.lines().next().unwrap().ends_with(VALID), "{text}");
    let output =
        run(sandbox
            .vpt()
            .current_dir(sandbox.path())
            .args(["doctor", "made-up-check", "--json"]));
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["kind"], "usage");
    assert!(error["error"]["message"].as_str().unwrap().ends_with(VALID));
}
