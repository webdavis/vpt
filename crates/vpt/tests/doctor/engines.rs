use crate::fixture::{
    advice, check, configure, configured, detail, issues, result, run_json, snapshot,
};
use std::os::unix::fs::PermissionsExt;

#[test]
fn default_engine_posture_reports_local_apple_and_unused_whisply() {
    let sandbox = configured("doctor-engine-default");
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "engines", "apple"),
        "selected, available, local"
    );
    assert_eq!(
        detail(&document, "engines", "whisply"),
        "unused, unavailable, local"
    );
    assert_eq!(result(&document, "engines", "whisply"), "ok");
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn command_locality_is_declared_and_commands_are_never_executed() {
    for local in [true, false] {
        let sandbox = configured("doctor-engine-command");
        let program = sandbox.path().join("bin/custom-engine");
        std::fs::write(
            &program,
            "#!/bin/sh\necho private-output\n/usr/bin/touch executed\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        configure(
            &sandbox,
            &format!(
                "[engines]\nmain = 'custom'\n[engines.custom]\nkind = 'command'\ncommand = ['custom-engine', 'private-argument']\nfamily = 'private-family'\nlocal = {local}\n"
            ),
        );
        let before = snapshot(&sandbox);
        let document = run_json(&sandbox, 0);
        let text = detail(&document, "engines", "custom");
        let posture = if local {
            "declared local"
        } else {
            "declared remote (audio leaves the machine)"
        };
        assert!(text == format!("selected, available, {posture}"), "{text}");
        assert!(!document.to_string().contains("private-"));
        assert_eq!(snapshot(&sandbox), before);
    }
}

#[test]
fn unavailable_language_override_fails_without_skipping_other_checks() {
    let sandbox = configured("doctor-engine-language");
    configure(
        &sandbox,
        "[engines.by_language.fr-FR]\nmain = 'remote'\n[engines.remote]\nkind = 'command'\ncommand = ['/absent/private-program', 'private-token']\nfamily = 'cloud'\nlocal = false\n",
    );
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    assert_eq!(check(&document, "engines")["result"], "issue");
    assert!(
        detail(&document, "engines", "remote")
            .starts_with("selected, unavailable, declared remote")
    );
    assert_eq!(
        advice(&document, "engines", "remote"),
        "make its program an executable file, or change `engines.remote` in the config"
    );
    assert_eq!(issues(&document), ["engines remote"]);
    assert!(!document.to_string().contains("private-"));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn checker_requires_an_executable_regular_file() {
    for kind in ["missing", "directory", "nonexecutable", "executable"] {
        let sandbox = configured("doctor-engine-checker");
        let program = sandbox.path().join("checker");
        match kind {
            "directory" => std::fs::create_dir(&program).unwrap(),
            "nonexecutable" | "executable" => {
                std::fs::write(&program, "#!/bin/sh\nexit 37\n").unwrap();
                let mode = if kind == "executable" { 0o700 } else { 0o600 };
                std::fs::set_permissions(&program, std::fs::Permissions::from_mode(mode)).unwrap();
            }
            _ => {}
        }
        configure(
            &sandbox,
            &format!(
                "[engines]\nchecker = 'whisply'\n[engines.whisply]\ncommand = [{program:?}]\n"
            ),
        );
        let before = snapshot(&sandbox);
        let document = run_json(&sandbox, if kind == "executable" { 0 } else { 1 });
        let expected = if kind == "executable" { "ok" } else { "issue" };
        assert_eq!(result(&document, "engines", "whisply"), expected);
        assert_eq!(snapshot(&sandbox), before);
    }
}

#[test]
fn apple_availability_requires_a_compatible_helper() {
    let sandbox = configured("doctor-engine-apple");
    let output = crate::support::run(
        sandbox
            .vpt()
            .current_dir(sandbox.path())
            .env("VPT_FAKE_VERSION", "2.0.0")
            .args(["doctor", "-o", "json"]),
    );
    let document = crate::fixture::report(&output, 1);
    assert_eq!(result(&document, "engines", "apple"), "issue");
    assert_eq!(
        detail(&document, "engines", "apple"),
        "selected, unavailable, local"
    );
}

#[test]
fn engines_end_with_a_not_tested_row_naming_the_test_command() {
    let sandbox = configured("doctor-engine-not-tested");
    let document = run_json(&sandbox, 0);
    let rows = check(&document, "engines")["rows"].as_array().unwrap();
    let last = rows.last().unwrap();
    assert_eq!(last["name"], "test");
    assert_eq!(last["result"], "not_run");
    assert_eq!(last["detail"], "not tested");
    assert_eq!(last["advice"], "run `vpt test engines`");
    assert_eq!(check(&document, "engines")["result"], "ok");
}
