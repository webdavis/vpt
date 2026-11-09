mod support;

use support::{Sandbox, broken_output, run, stderr, stdout};

#[test]
fn a_json_stdout_write_failure_is_one_store_document_on_stderr() {
    let sandbox = Sandbox::new("version-json-write-error");
    let (descriptor, _peer) = broken_output();
    let output = run(sandbox
        .vpt()
        .args(["--version", "--json"])
        .stdout(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    let document: serde_json::Value =
        serde_json::from_str(&stderr(&output)).expect("json on stderr");
    assert_eq!(document["schema"], "vpt.error/1");
    assert_eq!(document["error"]["kind"], "store");
    assert_eq!(document["error"]["message"], "standard output write failed");
}

#[test]
fn json_version_with_both_output_streams_broken_exits_one() {
    let sandbox = Sandbox::new("version-json-both-streams-broken");
    let (out, _peer) = broken_output();
    let err = out.try_clone().expect("stderr descriptor");
    let output = run(sandbox
        .vpt()
        .args(["--version", "--json"])
        .stdout(out)
        .stderr(err));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn version_reports_the_crate_version_and_no_helper_when_the_path_is_empty() {
    let sandbox = Sandbox::new("version-json");
    let output = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(document["schema"], "vpt.result/1");
    assert_eq!(document["command"], "version");
    assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
    assert!(document["helper_version"].is_null());
}

#[test]
fn version_without_json_prints_one_human_line() {
    let sandbox = Sandbox::new("version-human");
    let output = run(sandbox.vpt().arg("--version"));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        format!("vpt {} (helper absent)\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn a_stdout_write_failure_exits_one() {
    let sandbox = Sandbox::new("version-write-error");
    let (descriptor, _peer) = broken_output();
    let output = run(sandbox.vpt().arg("--version").stdout(descriptor));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "vpt: standard output write failed\n");
}

#[test]
fn version_with_both_output_streams_broken_exits_one() {
    let sandbox = Sandbox::new("version-both-streams-broken");
    let (out, _peer) = broken_output();
    let err = out.try_clone().expect("stderr descriptor");
    let output = run(sandbox.vpt().arg("--version").stdout(out).stderr(err));
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[cfg(feature = "dev-tools")]
#[test]
fn version_uses_path_helper_only_when_selected_configuration_is_absent() {
    let sandbox = Sandbox::new("version-fallback");
    sandbox.install_fake_helper();
    let output = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("one JSON document");
    assert_eq!(document["helper_version"], "1.0.0");
}

#[cfg(feature = "dev-tools")]
#[test]
fn version_honors_environment_configuration_and_explicit_override() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new("version-selected");
    sandbox.install_fake_helper();
    std::fs::create_dir_all(sandbox.config_path().parent().expect("parent")).expect("config dir");
    let missing = sandbox.path().join("missing-helper");
    let settings = format!(
        "config_version = 1\n[helper]\npath = {}\n",
        serde_json::to_string(&missing.to_str().expect("utf8")).expect("path")
    );
    std::fs::write(sandbox.config_path(), settings).expect("selected config");
    std::fs::set_permissions(
        sandbox.config_path().parent().unwrap(),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::set_permissions(
        sandbox.config_path(),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let configured = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(configured.status.code(), Some(0));
    let document: serde_json::Value =
        serde_json::from_str(&stdout(&configured)).expect("one JSON document");
    assert!(document["helper_version"].is_null());

    let alternate = sandbox.config_path().with_file_name("alternate.toml");
    let helper = sandbox.path().join("bin/vpt-macos");
    let settings = format!(
        "config_version = 1\n[helper]\npath = {}\n",
        serde_json::to_string(&helper.to_str().expect("utf8")).expect("path")
    );
    std::fs::write(&alternate, settings).expect("alternate config");
    std::fs::set_permissions(&alternate, std::fs::Permissions::from_mode(0o600)).unwrap();
    let output = run(sandbox
        .vpt()
        .args(["--version", "--json", "--config"])
        .arg(&alternate));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("one JSON document");
    assert_eq!(document["helper_version"], "1.0.0");
}

#[cfg(feature = "dev-tools")]
#[test]
fn invalid_existing_configuration_keeps_version_success_without_path_fallback() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new("version-invalid");
    sandbox.install_fake_helper();
    std::fs::create_dir_all(sandbox.config_path().parent().expect("parent")).expect("config dir");
    std::fs::write(sandbox.config_path(), "CANARY-INVALID-CONFIG\n[").expect("invalid config");
    std::fs::set_permissions(
        sandbox.config_path().parent().unwrap(),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::set_permissions(
        sandbox.config_path(),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let output = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let text = stdout(&output);
    let document: serde_json::Value = serde_json::from_str(&text).expect("one JSON document");
    assert!(document["helper_version"].is_null());
    assert!(!text.contains("CANARY"));
}

#[cfg(feature = "dev-tools")]
#[test]
fn unreadable_existing_configuration_keeps_version_success_without_path_fallback() {
    let sandbox = Sandbox::new("version-unreadable");
    sandbox.install_fake_helper();
    std::fs::create_dir_all(sandbox.config_path()).expect("nonregular config leaf");
    let output = run(sandbox.vpt().args(["--version", "--json"]));
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stderr(&output), "");
    let document: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("one JSON document");
    assert!(document["helper_version"].is_null());
}
