#[path = "doctor/active_writer.rs"]
mod active_writer;
#[path = "doctor/fixture.rs"]
mod fixture;
#[path = "doctor/independence.rs"]
mod independence;
#[path = "doctor/observations.rs"]
mod observations;
#[path = "show_list_storage/fixture.rs"]
mod observations_fixture;
#[allow(
    dead_code,
    reason = "shared integration support serves other test targets"
)]
mod support;

use fixture::{configured, detail, report, run_json, snapshot};
use support::{run, stderr, stdout};

#[test]
fn healthy_doctor_reports_all_eighteen_checks() {
    let sandbox = configured("doctor-healthy");
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 0);
    assert!(checks.iter().all(|check| check["ok"] == true));
    assert_eq!(detail(&checks, "symlink"), "not configured");
    assert_eq!(detail(&checks, "cleanup_pending"), "none");
    assert_eq!(
        detail(&checks, "source_gone"),
        "0 recordings whose source is gone"
    );
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn healthy_human_doctor_lists_every_check() {
    let sandbox = configured("doctor-human");
    let output = run(sandbox.vpt().current_dir(sandbox.path()).arg("doctor"));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let text = stdout(&output);
    assert_eq!(text.lines().count(), 18);
    for name in fixture::NAMES {
        assert!(
            text.lines()
                .any(|line| line.split_whitespace().nth(1) == Some(name)),
            "missing {name}: {text}"
        );
    }
}

#[test]
fn missing_helper_reports_both_install_commands() {
    let sandbox = configured("doctor-helper-absent");
    std::fs::rename(
        sandbox.path().join("bin/vpt-macos"),
        sandbox.path().join("held-helper"),
    )
    .unwrap();
    let checks = run_json(&sandbox, 3);
    let text = detail(&checks, "helper");
    assert!(text.contains("cargo install --git https://github.com/webdavis/vpt vpt"));
    assert!(text.contains("swift build -c release"));
    let human = run(sandbox.vpt().current_dir(sandbox.path()).arg("doctor"));
    assert_eq!(human.status.code(), Some(3));
    assert!(human.stdout.is_empty());
    assert!(stderr(&human).contains("FAIL helper"));
    assert!(stderr(&human).contains("ok   config"));
}

#[test]
fn compatible_helper_keeps_additive_diagnostics() {
    helper_diagnostics("1.0.0", 0);
}
#[test]
fn incompatible_helper_keeps_additive_diagnostics() {
    helper_diagnostics("2.0.0", 3);
}

fn helper_diagnostics(version: &str, code: i32) {
    let sandbox = configured("doctor-helper-diagnostics");
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .env("VPT_FAKE_VERSION", version)
        .env("VPT_FAKE_VERSION_MODE", "additive")
        .args(["doctor", "--json"]));
    let checks = report(&output, code);
    assert_eq!(fixture::check(&checks, "helper")["ok"], code == 0);
    assert!(detail(&checks, "helper").contains("unknown additive helper field: capabilities"));
}

#[test]
fn helper_failure_never_exposes_child_streams() {
    let sandbox = configured("doctor-helper-streams");
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .env("VPT_FAKE_VERSION_MODE", "private-failure")
        .args(["doctor", "--json"]));
    let checks = report(&output, 3);
    assert!(
        !checks
            .iter()
            .any(|check| check.to_string().contains("private-"))
    );
    assert_eq!(fixture::check(&checks, "helper")["ok"], false);
}

#[test]
fn missing_config_reports_setup_and_all_dependencies() {
    let sandbox = configured("doctor-no-config");
    std::fs::rename(sandbox.config_path(), sandbox.path().join("held-config")).unwrap();
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 3);
    assert!(detail(&checks, "config").contains("vpt setup"));
    for check in checks.iter().filter(|check| check["name"] != "config") {
        assert_eq!(check["ok"], false);
        assert_eq!(check["detail"], "not run: the configuration did not load");
    }
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn malformed_config_never_exposes_contents() {
    let sandbox = configured("doctor-malformed");
    std::fs::write(sandbox.config_path(), "private-config-canary = [").unwrap();
    let checks = run_json(&sandbox, 3);
    assert!(
        !serde_json::to_string(&checks)
            .unwrap()
            .contains("private-config-canary")
    );
}

#[test]
fn config_path_selection_failure_remains_aggregate() {
    use std::os::unix::ffi::OsStringExt;
    let sandbox = configured("doctor-config-selection");
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .env("VPT_CONFIG", std::ffi::OsString::from_vec(vec![0xff]))
        .args(["doctor", "--json"]));
    let checks = report(&output, 3);
    assert_eq!(fixture::check(&checks, "config")["ok"], false);
    assert_eq!(
        detail(&checks, "helper"),
        "not run: the configuration did not load"
    );
}
