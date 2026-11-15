#[path = "doctor/active_writer.rs"]
mod active_writer;
#[path = "doctor/engines.rs"]
mod engines;
#[path = "doctor/fixture.rs"]
mod fixture;
#[path = "doctor/human.rs"]
mod human;
#[path = "doctor/independence.rs"]
mod independence;
#[path = "doctor/kept.rs"]
mod kept;
#[path = "doctor/named.rs"]
mod named;
#[path = "doctor/observations.rs"]
mod observations;
#[path = "show_list_storage/fixture.rs"]
mod observations_fixture;
#[path = "doctor/source.rs"]
mod source;
#[path = "doctor/speakers.rs"]
mod speakers;
#[path = "doctor/state_dir.rs"]
mod state_dir;
#[path = "doctor/summary.rs"]
mod summary;
#[allow(
    dead_code,
    reason = "shared integration support serves other test targets"
)]
mod support;
#[path = "doctor/vault.rs"]
mod vault;

use fixture::{
    CHECKS, advice, check, configured, detail, issues, names, report, result, row, run_json,
    snapshot,
};
use support::{run, stderr, stdout};

#[test]
fn healthy_doctor_reports_every_check_in_order_and_exits_zero() {
    let sandbox = configured("doctor-healthy");
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(names(&document), CHECKS);
    assert_eq!(issues(&document), Vec::<String>::new());
    assert_eq!(detail(&document, "stores", "home_link"), "not configured");
    assert_eq!(detail(&document, "leftovers", "staged_audio"), "none");
    assert_eq!(
        detail(&document, "ledger", "source_gone"),
        "0 recordings whose source is gone"
    );
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn warnings_alone_keep_exit_zero() {
    let sandbox = configured("doctor-warnings");
    let document = run_json(&sandbox, 0);
    assert_eq!(result(&document, "stores", "audio"), "warning");
    assert_eq!(check(&document, "stores")["result"], "warning");
}

#[test]
fn missing_helper_is_an_issue_with_both_install_commands() {
    let sandbox = configured("doctor-helper-absent");
    std::fs::rename(
        sandbox.path().join("bin/vpt-macos"),
        sandbox.path().join("held-helper"),
    )
    .unwrap();
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "helper", "vpt-macos"), "issue");
    let text = advice(&document, "helper", "vpt-macos");
    assert!(text.contains("cargo install --git https://github.com/webdavis/vpt vpt"));
    assert!(text.contains("swift build -c release"));
    let human = run(sandbox.vpt().current_dir(sandbox.path()).arg("doctor"));
    assert_eq!(human.status.code(), Some(1));
    assert!(human.stderr.is_empty(), "{}", stderr(&human));
    assert!(
        stdout(&human).contains("\n  ✗ vpt-macos: "),
        "{}",
        stdout(&human)
    );
}

#[test]
fn compatible_helper_keeps_additive_diagnostics() {
    helper_diagnostics("1.0.0", 0);
}
#[test]
fn incompatible_helper_keeps_additive_diagnostics() {
    helper_diagnostics("2.0.0", 1);
}

fn helper_diagnostics(version: &str, code: i32) {
    let sandbox = configured("doctor-helper-diagnostics");
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .env("VPT_FAKE_VERSION", version)
        .env("VPT_FAKE_VERSION_MODE", "additive")
        .args(["doctor", "--json"]));
    let document = report(&output, code);
    let expected = if code == 0 { "ok" } else { "issue" };
    assert_eq!(result(&document, "helper", "vpt-macos"), expected);
    assert!(
        detail(&document, "helper", "vpt-macos")
            .contains("unknown additive helper field: capabilities")
    );
}

#[test]
fn helper_failure_never_exposes_child_streams() {
    let sandbox = configured("doctor-helper-streams");
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .env("VPT_FAKE_VERSION_MODE", "private-failure")
        .args(["doctor", "--json"]));
    let document = report(&output, 1);
    assert!(!document.to_string().contains("private-"));
    assert_eq!(result(&document, "helper", "vpt-macos"), "issue");
}

#[test]
fn missing_config_is_an_issue_and_every_other_check_is_not_run() {
    let sandbox = configured("doctor-no-config");
    std::fs::rename(sandbox.config_path(), sandbox.path().join("held-config")).unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "config", "file"), "issue");
    assert!(advice(&document, "config", "file").contains("vpt setup"));
    for name in &CHECKS[1..] {
        let skipped = check(&document, name);
        assert_eq!(skipped["result"], "not_run", "{name}");
        assert_eq!(skipped["rows"], serde_json::json!([]), "{name}");
        assert_eq!(
            skipped["advice"],
            "the configuration did not load; see `vpt doctor config`"
        );
    }
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn malformed_config_never_exposes_contents() {
    let sandbox = configured("doctor-malformed");
    std::fs::write(sandbox.config_path(), "private-config-canary = [").unwrap();
    let document = run_json(&sandbox, 1);
    assert!(!document.to_string().contains("private-config-canary"));
}

#[test]
fn config_path_selection_failure_skips_the_other_checks() {
    use std::os::unix::ffi::OsStringExt;
    let sandbox = configured("doctor-config-selection");
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .env("VPT_CONFIG", std::ffi::OsString::from_vec(vec![0xff]))
        .args(["doctor", "--json"]));
    let document = report(&output, 1);
    assert_eq!(row(&document, "config", "file")["result"], "issue");
    assert_eq!(check(&document, "helper")["result"], "not_run");
}
