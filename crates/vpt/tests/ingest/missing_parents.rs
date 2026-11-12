use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn configure_roots(sandbox: &Sandbox, stores: &[(&str, &Path)]) {
    let state = sandbox.path().join("home/.local/state/vpt");
    let mut config = fs::read_to_string(sandbox.config_path()).unwrap().replace(
        sandbox.path().join("state/vpt").to_str().unwrap(),
        state.to_str().unwrap(),
    );
    config.push_str("[stores]\n");
    for (key, path) in stores {
        config.push_str(&format!("{key} = {path:?}\n"));
    }
    fs::write(sandbox.config_path(), config).unwrap();
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn ingest_creates_every_missing_folder_of_configured_roots_privately() {
    let sandbox = configured();
    let vault = sandbox.path().join("vault");
    fs::create_dir(&vault).unwrap();
    fs::set_permissions(&vault, fs::Permissions::from_mode(0o755)).unwrap();
    let nested = vault.join("vpt");
    configure_roots(
        &sandbox,
        &[
            ("transcripts", &nested.join("transcripts")),
            ("analysis", &nested.join("analysis")),
            ("briefs", &nested.join("briefs")),
        ],
    );
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    for leaf in [
        "vault/vpt",
        "vault/vpt/transcripts",
        "vault/vpt/analysis",
        "vault/vpt/briefs",
        "home/.local",
        "home/.local/state",
        "home/.local/state/vpt",
    ] {
        assert_eq!(mode(&sandbox.path().join(leaf)), 0o700, "{leaf}");
    }
    assert_eq!(mode(&vault), 0o755);
}

#[test]
fn parent_traversal_below_a_missing_folder_is_refused_without_writes() {
    let sandbox = configured();
    let evil = sandbox.path().join("missing/../voice-memos/evil");
    configure_roots(&sandbox, &[("audio", &evil)]);
    let before = snapshot(&sandbox);
    let output = run(sandbox.vpt().args(["ingest", "--json"]));
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_str(&stderr(&output)).unwrap();
    assert_eq!(error["error"]["kind"], "config");
    assert!(stderr(&output).contains("stores.audio"));
    assert_eq!(snapshot(&sandbox), before);
}
