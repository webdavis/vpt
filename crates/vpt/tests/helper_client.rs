#![cfg(feature = "dev-tools")]

#[allow(dead_code, reason = "shared fixture has consumers in other test binaries")]
mod support;

use support::Sandbox;
use vpt_adapters::{BUILT_AGAINST_MAJOR, HelperClient, HelperError};
use vpt_application::ports::TrashError;

impl Sandbox {
    fn helper_env(&self) -> Vec<(&'static str, String)> {
        [
            ("HOME", self.path().join("home")),
            ("XDG_CONFIG_HOME", self.path().join("config")),
            ("XDG_DATA_HOME", self.path().join("data")),
            ("XDG_STATE_HOME", self.path().join("state")),
            ("XDG_CACHE_HOME", self.path().join("cache")),
            ("TMPDIR", self.path().join("tmp")),
            ("VPT_CONFIG", self.config_path()),
            ("PATH", self.path().join("bin")),
            ("VPT_FAKE_TRASH", self.path().join("trash")),
            ("VPT_FAKE_LOG", self.fake_log()),
        ]
        .into_iter()
        .map(|(key, value)| (key, value.to_string_lossy().into_owned()))
        .collect()
    }
}

fn env_pairs<'a>(vars: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    vars.iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect()
}

#[test]
fn direct_fake_version_uses_explicit_sandbox_environment() {
    let sandbox = Sandbox::new("helper-version");
    sandbox.install_fake_helper();
    let vars = sandbox.helper_env();
    let env = env_pairs(&vars);
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    assert_eq!(client.version_with_env(&env), Ok("1.0.0".into()));
    assert_eq!(BUILT_AGAINST_MAJOR, 1);
}

#[test]
fn absent_helper_is_typed_without_touching_a_sandbox_target() {
    let sandbox = Sandbox::new("helper-absent");
    let victim = sandbox.path().join("victim");
    std::fs::write(&victim, b"fixture").expect("victim");
    let vars = sandbox.helper_env();
    let env = env_pairs(&vars);
    let client = HelperClient::new(sandbox.path().join("missing-helper"));
    assert_eq!(client.version_with_env(&env), Err(HelperError::Absent));
    assert_eq!(
        client.trash_with_env(&victim, &env),
        Err(TrashError::HelperAbsent)
    );
    assert_eq!(std::fs::read(&victim).expect("unmoved"), b"fixture");
    assert!(!sandbox.fake_log().exists());
}

#[test]
fn fake_trash_moves_only_the_owned_fixture_after_compatibility() {
    let sandbox = Sandbox::new("helper-trash");
    sandbox.install_fake_helper();
    let victim = sandbox.path().join("victim.txt");
    std::fs::write(&victim, b"fixture").expect("victim");
    let vars = sandbox.helper_env();
    let env = env_pairs(&vars);
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    assert_eq!(client.trash_with_env(&victim, &env), Ok(victim.clone()));
    assert!(!victim.exists());
    assert_eq!(
        std::fs::read(sandbox.path().join("trash/victim.txt")).expect("fake trash"),
        b"fixture"
    );
    let log = std::fs::read_to_string(sandbox.fake_log()).expect("fake log");
    assert!(log.contains("\"command\":\"trash\""));
}

#[test]
fn mismatched_helper_never_receives_a_trash_request() {
    let sandbox = Sandbox::new("helper-mismatch");
    sandbox.install_fake_helper();
    let victim = sandbox.path().join("victim.txt");
    std::fs::write(&victim, b"fixture").expect("victim");
    let mut vars = sandbox.helper_env();
    vars.push(("VPT_FAKE_VERSION", "2.0.0".into()));
    let env = env_pairs(&vars);
    let client = HelperClient::new(sandbox.path().join("bin/vpt-macos"));
    assert_eq!(
        client.trash_with_env(&victim, &env),
        Err(TrashError::HelperVersion { found: 2 })
    );
    assert_eq!(std::fs::read(&victim).expect("unmoved"), b"fixture");
    assert!(!sandbox.path().join("trash/victim.txt").exists());
    assert!(!sandbox.fake_log().exists());
}
