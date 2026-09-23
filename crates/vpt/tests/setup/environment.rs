use super::support;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use support::{Sandbox, run, stderr, stdout};

fn invalid() -> OsString {
    OsString::from_vec(b"PRIVATE-environment-\xff".to_vec())
}

#[test]
fn unrelated_non_utf8_environment_is_ignored() {
    let sandbox = Sandbox::new("unrelated-env");
    let output = run(sandbox
        .vpt()
        .args(["setup", "--json"])
        .env("UNRELATED", invalid())
        .env(invalid(), "unrelated"));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("needs a controlling terminal"),
        "{}",
        stderr(&output)
    );
    assert_eq!(stdout(&output), "");
}
#[test]
fn invalid_selected_environment_is_private_and_never_falls_back() {
    for key in ["VPT_CONFIG", "XDG_CONFIG_HOME", "HOME", "XDG_STATE_HOME"] {
        let sandbox = Sandbox::new("selected-env");
        let mut command = sandbox.vpt();
        if key == "XDG_CONFIG_HOME" {
            command.env_remove("VPT_CONFIG");
        }
        let output = run(command.args(["setup", "--json"]).env(key, invalid()));
        assert_eq!(output.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_str(&stderr(&output)).expect("error");
        assert_eq!(error["error"]["kind"], "config");
        assert!(stderr(&output).contains(key), "{}", stderr(&output));
        assert!(!stderr(&output).contains("PRIVATE"));
        assert!(!sandbox.config_path().exists());
        assert!(!sandbox.path().join("home/.vpt").exists());
    }
}
#[test]
fn explicit_config_overrides_invalid_vpt_config() {
    let sandbox = Sandbox::new("explicit-env");
    let path = sandbox.path().join("config/selected.toml");
    std::fs::set_permissions(
        path.parent().expect("parent"),
        std::fs::Permissions::from_mode(0o700),
    )
    .expect("private config parent");
    std::fs::write(&path, "kept").expect("existing");
    let output = run(sandbox
        .vpt()
        .args(["setup", "--config"])
        .arg(&path)
        .env("VPT_CONFIG", invalid())
        .env("XDG_CONFIG_HOME", invalid()));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("selected.toml exists"),
        "{}",
        stderr(&output)
    );
}
#[test]
fn vpt_config_overrides_invalid_xdg_config_home() {
    let sandbox = Sandbox::new("vpt-env");
    let path = sandbox.path().join("config/selected.toml");
    std::fs::set_permissions(
        path.parent().expect("parent"),
        std::fs::Permissions::from_mode(0o700),
    )
    .expect("private config parent");
    std::fs::write(&path, "kept").expect("existing");
    let output = run(sandbox
        .vpt()
        .arg("setup")
        .env("VPT_CONFIG", path)
        .env("XDG_CONFIG_HOME", invalid()));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("selected.toml exists"),
        "{}",
        stderr(&output)
    );
}
#[test]
fn invalid_selected_state_path_is_a_key_only_config_error() {
    let sandbox = Sandbox::new("invalid-state");
    let output = run(sandbox
        .vpt()
        .args(["setup", "--json"])
        .env("XDG_STATE_HOME", "PRIVATE-relative"));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("home.state_dir"),
        "{}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("PRIVATE"));
    assert!(!sandbox.config_path().exists());
}
