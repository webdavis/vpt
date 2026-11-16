use super::support::{Sandbox, run, stderr, stdout};
use std::os::unix::fs::PermissionsExt;

const NO_TERMINAL: &str =
    "vpt setup needs a terminal to choose the engine; pass --engine apple or --engine whisply";

#[test]
fn setup_with_an_engine_writes_the_config_without_a_terminal() {
    for engine in ["apple", "whisply"] {
        let sandbox = Sandbox::new("setup-engine");
        let output = run(sandbox
            .vpt()
            .args(["setup", "--engine", engine, "-o", "json"]));
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let result: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
        assert_eq!(result["main_engine"], engine);
        let config = std::fs::read_to_string(sandbox.config_path()).unwrap();
        assert!(config.contains(&format!("main = \"{engine}\"")), "{config}");
        assert!(sandbox.path().join("home/.vpt/audio").is_dir());
        assert!(sandbox.path().join("state/vpt").is_dir());
    }
}

#[test]
fn setup_refuses_an_engine_it_does_not_offer() {
    let sandbox = Sandbox::new("setup-engine-unknown");
    let output = run(sandbox.vpt().args(["setup", "--engine", "other"]));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("apple"), "{}", stderr(&output));
    assert!(!sandbox.config_path().exists());
}

#[test]
fn setup_without_an_engine_or_a_terminal_names_the_flag() {
    let sandbox = Sandbox::new("setup-engine-missing");
    let output = run(sandbox.vpt().args(["setup", "-o", "json"]));
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["message"], NO_TERMINAL);
    assert!(!sandbox.config_path().exists());
}

#[test]
fn setup_with_an_engine_still_refuses_an_existing_config() {
    let sandbox = Sandbox::new("setup-engine-existing");
    let parent = sandbox.config_path().parent().unwrap().to_path_buf();
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(sandbox.config_path(), "original").unwrap();
    let output = run(sandbox.vpt().args(["setup", "--engine", "whisply"]));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("pass --overwrite"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.config_path()).unwrap(),
        "original"
    );
}
