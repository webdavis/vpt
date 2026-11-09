use crate::fixture::{check, configure, configured, detail, run_json, snapshot};
use std::os::unix::fs::PermissionsExt;

#[test]
fn default_engine_posture_reports_local_apple_and_unused_whisply() {
    let sandbox = configured("doctor-engine-default");
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 0);
    let text = detail(&checks, "engines");
    assert!(text.contains("apple: selected, available, local"), "{text}");
    assert!(
        text.contains("whisply: unused, unavailable, local"),
        "{text}"
    );
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
        let checks = run_json(&sandbox, 0);
        let text = detail(&checks, "engines");
        let posture = if local {
            "declared local"
        } else {
            "declared remote (audio leaves the machine)"
        };
        assert!(
            text.contains(&format!("custom: selected, available, {posture}")),
            "{text}"
        );
        assert!(!text.contains("private-"));
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
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "engines")["ok"], false);
    assert!(detail(&checks, "engines").contains("remote: selected, unavailable, declared remote"));
    for name in ["helper", "stores", "vault", "source_gone", "recordings_dir"] {
        assert_eq!(check(&checks, name)["ok"], true, "{name}");
    }
    assert!(!serde_json::to_string(&checks).unwrap().contains("private-"));
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
        let checks = run_json(&sandbox, if kind == "executable" { 0 } else { 3 });
        assert_eq!(check(&checks, "engines")["ok"], kind == "executable");
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
            .args(["doctor", "--json"]),
    );
    let checks = crate::fixture::report(&output, 3);
    assert_eq!(check(&checks, "engines")["ok"], false);
    assert!(detail(&checks, "engines").contains("apple: selected, unavailable, local"));
}
