#[path = "symlink/fixture.rs"]
mod fixture;
mod support;

use fixture::{configured, error, snapshot, write_config};
use support::{Sandbox, broken_output, run, stderr, stdout};

#[test]
fn deploy_reports_the_link_and_target_and_verify_succeeds() {
    let (sandbox, target) = configured();
    let output = run(sandbox.vpt().args(["symlink", "deploy", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(output.stderr.is_empty());
    let document: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("result JSON");
    assert_eq!(document["schema"], "vpt.result/1");
    assert_eq!(document["command"], "symlink deploy");
    assert_eq!(
        document["link"],
        sandbox.path().join("home/.vpt").to_string_lossy().as_ref()
    );
    assert_eq!(document["target"], target.to_string_lossy().as_ref());
    assert_eq!(document["ok"], true);
    let before = snapshot(sandbox.path());
    let verify = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    assert_eq!(verify.status.code(), Some(0), "{}", stderr(&verify));
    let verified: serde_json::Value = serde_json::from_str(&stdout(&verify)).expect("result JSON");
    assert_eq!(verified["command"], "symlink verify");
    assert_eq!(verified["ok"], true);
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn missing_link_verification_reports_refusal_and_writes_nothing() {
    let (sandbox, _) = configured();
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    let document = error(&output, 3, "refused");
    assert_eq!(document["error"]["rule"], "symlink_verify");
    assert!(
        document["error"]["message"]
            .as_str()
            .expect("message")
            .contains("does not exist")
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn occupied_directory_deployment_reports_refusal_and_preserves_everything() {
    let (sandbox, _) = configured();
    std::fs::create_dir(sandbox.path().join("home/.vpt")).expect("occupied");
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "deploy", "--json"]));
    let document = error(&output, 3, "refused");
    assert_eq!(document["error"]["rule"], "symlink_deploy");
    assert!(
        document["error"]["message"]
            .as_str()
            .expect("message")
            .contains("a directory")
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn dangling_link_verification_does_not_repair_the_target() {
    let (sandbox, target) = configured();
    std::os::unix::fs::symlink(&target, sandbox.path().join("home/.vpt")).expect("dangling link");
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    assert_eq!(
        error(&output, 3, "refused")["error"]["rule"],
        "symlink_verify"
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn deploy_without_a_target_is_a_usage_error() {
    let sandbox = Sandbox::new("symlink-unset-deploy");
    write_config(&sandbox.config_path(), "config_version = 1\n");
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "deploy", "--json"]));
    assert_eq!(
        error(&output, 2, "usage")["error"]["message"],
        "home.symlink_target is not set"
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn verify_without_a_target_is_a_usage_error() {
    let sandbox = Sandbox::new("symlink-unset-verify");
    write_config(&sandbox.config_path(), "config_version = 1\n");
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    assert_eq!(
        error(&output, 2, "usage")["error"]["message"],
        "home.symlink_target is not set"
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn explicit_configuration_overrides_the_environment_path() {
    let (sandbox, _) = configured();
    let alternate = sandbox.path().join("config/alternate.toml");
    write_config(&alternate, "config_version = 1\n");
    let before = snapshot(sandbox.path());
    let output = run(sandbox
        .vpt()
        .args(["symlink", "deploy", "--json", "--config"])
        .arg(&alternate));
    assert_eq!(
        error(&output, 2, "usage")["error"]["message"],
        "home.symlink_target is not set"
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn deployment_uses_only_settings_without_opening_other_roots() {
    let (sandbox, target) = configured();
    let text = format!(
        "config_version = 1\n[home]\nsymlink_target = {:?}\nstate_dir = {:?}\n[source]\nrecordings_dir = {:?}\n[helper]\npath = {:?}\n",
        target,
        sandbox.path().join("missing/state"),
        sandbox.path().join("missing/source"),
        sandbox.path().join("missing/helper")
    );
    write_config(&sandbox.config_path(), &text);
    let output = run(sandbox.vpt().args(["symlink", "deploy"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!(
            "{} -> {}\n",
            sandbox.path().join("home/.vpt").display(),
            target.display()
        )
    );
    assert!(!sandbox.path().join("missing").exists());
    assert_eq!(
        std::fs::read_dir(sandbox.path().join("state"))
            .expect("state")
            .count(),
        0
    );
}

#[test]
fn configured_target_requires_the_default_home() {
    let (sandbox, target) = configured();
    write_config(
        &sandbox.config_path(),
        &format!(
            "config_version = 1\n[home]\npath = {:?}\nsymlink_target = {:?}\n",
            sandbox.path().join("custom-home"),
            target
        ),
    );
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "deploy", "--json"]));
    let document = error(&output, 2, "config");
    assert!(
        document["error"]["message"]
            .as_str()
            .expect("message")
            .contains("home.path")
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn relative_target_is_a_configuration_error_without_writes() {
    let sandbox = Sandbox::new("symlink-relative");
    write_config(
        &sandbox.config_path(),
        "config_version = 1\n[home]\nsymlink_target = 'relative/target'\n",
    );
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "deploy", "--json"]));
    let document = error(&output, 2, "config");
    assert!(
        document["error"]["message"]
            .as_str()
            .expect("message")
            .contains("absolute")
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn missing_configuration_reports_the_selected_path_without_writes() {
    let sandbox = Sandbox::new("symlink-missing-config");
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    let document = error(&output, 2, "config");
    assert!(
        document["error"]["message"]
            .as_str()
            .expect("message")
            .contains(sandbox.config_path().to_str().expect("fixture path"))
    );
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn invalid_configuration_does_not_echo_its_contents() {
    let sandbox = Sandbox::new("symlink-invalid-config");
    write_config(
        &sandbox.config_path(),
        "config_version = 1\nsecret-canary = 'unterminated",
    );
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    error(&output, 2, "config");
    assert!(!stderr(&output).contains("secret-canary"));
    assert!(stderr(&output).contains("configuration does not parse"));
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn configuration_leaf_symlink_is_refused_without_following_it() {
    let sandbox = Sandbox::new("symlink-config-link");
    let selected = sandbox.path().join("config/selected.toml");
    write_config(&selected, "config_version = 1\n");
    std::fs::create_dir_all(sandbox.config_path().parent().expect("parent")).expect("parent");
    std::os::unix::fs::symlink(&selected, sandbox.config_path()).expect("config link");
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "verify", "--json"]));
    assert_eq!(error(&output, 3, "refused")["error"]["rule"], "path_escape");
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn target_io_failure_is_a_safe_store_error_without_writes() {
    let sandbox = Sandbox::new("symlink-target-io");
    let invalid = format!("{}/private-canary\0", sandbox.path().display());
    let quoted = serde_json::to_string(&invalid).expect("quoted path");
    write_config(
        &sandbox.config_path(),
        &format!("config_version = 1\n[home]\nsymlink_target = {quoted}\n"),
    );
    let before = snapshot(sandbox.path());
    let output = run(sandbox.vpt().args(["symlink", "deploy", "--json"]));
    let document = error(&output, 1, "store");
    assert!(document["error"]["rule"].is_null());
    assert!(!stderr(&output).contains("private-canary"));
    assert_eq!(snapshot(sandbox.path()), before);
}

#[test]
fn existing_link_target_canonicalization_io_is_a_safe_store_error() {
    let (sandbox, _) = configured();
    let invalid = format!("{}/home/notes/private-canary\0", sandbox.path().display());
    let quoted = serde_json::to_string(&invalid).expect("quoted path");
    write_config(
        &sandbox.config_path(),
        &format!("config_version = 1\n[home]\nsymlink_target = {quoted}\n"),
    );
    std::os::unix::fs::symlink("notes/vpt", sandbox.path().join("home/.vpt"))
        .expect("existing link");
    let before = snapshot(sandbox.path());
    for verb in ["deploy", "verify"] {
        let output = run(sandbox.vpt().args(["symlink", verb, "--json"]));
        let document = error(&output, 1, "store");
        assert!(document["error"]["rule"].is_null());
        assert!(!stderr(&output).contains("private-canary"));
        assert_eq!(snapshot(sandbox.path()), before);
    }
}

#[test]
fn matching_link_with_nondirectory_target_parent_is_refused_without_writes() {
    let (sandbox, parent) = configured();
    std::fs::write(&parent, b"parent-canary").expect("regular parent");
    let target = parent.join("child");
    write_config(
        &sandbox.config_path(),
        &format!("config_version = 1\n[home]\nsymlink_target = {target:?}\n"),
    );
    std::os::unix::fs::symlink(&target, sandbox.path().join("home/.vpt")).expect("matching link");
    let before = snapshot(sandbox.path());
    for verb in ["deploy", "verify"] {
        let output = run(sandbox.vpt().args(["symlink", verb, "--json"]));
        let document = error(&output, 3, "refused");
        assert_eq!(document["error"]["rule"], format!("symlink_{verb}"));
        assert!(
            document["error"]["message"]
                .as_str()
                .expect("message")
                .contains("not a directory")
        );
        assert_eq!(snapshot(sandbox.path()), before);
    }
}

#[test]
fn deploy_output_failure_reports_store_error_and_preserves_the_created_link() {
    let (sandbox, target) = configured();
    let (writer, _peer) = broken_output();
    let output = run(sandbox
        .vpt()
        .args(["symlink", "deploy", "--json"])
        .stdout(writer));
    let document = error(&output, 1, "store");
    assert_eq!(document["error"]["message"], "standard output write failed");
    assert_eq!(
        std::fs::read_link(sandbox.path().join("home/.vpt")).expect("completed link"),
        target.canonicalize().expect("canonical target")
    );
}
