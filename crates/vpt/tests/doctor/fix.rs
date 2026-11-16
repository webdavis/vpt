use crate::fixture::{advice, configured, detail, report, result, snapshot};
use crate::support::{Sandbox, run, stderr};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;

const STORES: [&str; 7] = [
    "audio",
    "transcripts",
    "analysis",
    "briefs",
    "engine_outputs",
    "drafts",
    "released",
];

pub(super) fn fix(sandbox: &Sandbox, checks: &[&str], code: i32) -> Value {
    report(
        &run(sandbox
            .vpt()
            .current_dir(sandbox.path())
            .arg("doctor")
            .args(checks)
            .args(["--fix", "-o", "json"])),
        code,
    )
}

pub(super) fn fixed(document: &Value) -> Vec<String> {
    document["fixed"]
        .as_array()
        .unwrap_or_else(|| panic!("no fixed array: {document}"))
        .iter()
        .map(|entry| {
            format!(
                "{} {}",
                entry["check"].as_str().unwrap(),
                entry["name"].as_str().unwrap()
            )
        })
        .collect()
}

fn mode(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn fix_on_an_empty_machine_creates_the_starter_config_and_private_folders() {
    let sandbox = Sandbox::new("doctor-fix-empty");
    let document = fix(&sandbox, &[], 1);
    let mut expected = vec!["config file", "stores home", "stores state_dir"];
    let stores: Vec<String> = STORES.iter().map(|name| format!("stores {name}")).collect();
    expected.extend(stores.iter().map(String::as_str));
    assert_eq!(fixed(&document), expected);
    let config = sandbox.config_path();
    assert_eq!(mode(&config), 0o600);
    assert_eq!(
        detail(&document, "config", "file"),
        format!("created {}", config.display())
    );
    assert_eq!(
        document["fixed"][0],
        json!({"check": "config", "name": "file",
        "detail": format!("created {}", config.display())})
    );
    let home = sandbox.path().join("home/.vpt");
    assert_eq!(mode(&home), 0o700);
    assert_eq!(mode(&sandbox.path().join("state/vpt")), 0o700);
    assert!(detail(&document, "stores", "audio").starts_with("created "));
    for leaf in ["audio", "engine-outputs", "released"] {
        assert_eq!(mode(&home.join(leaf)), 0o700, "{leaf}");
    }
}

#[test]
fn fix_writes_the_same_config_text_as_setup_with_the_apple_engine() {
    let fixed_sandbox = Sandbox::new("doctor-fix-text");
    fix(&fixed_sandbox, &["config"], 0);
    let setup_sandbox = Sandbox::new("doctor-fix-setup-text");
    let output = run(setup_sandbox.vpt().args(["setup", "--engine", "apple"]));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = |sandbox: &Sandbox| {
        std::fs::read_to_string(sandbox.config_path())
            .unwrap()
            .replace(&sandbox.path().display().to_string(), "SANDBOX")
    };
    assert_eq!(text(&fixed_sandbox), text(&setup_sandbox));
}

#[test]
fn a_second_fix_changes_nothing_and_fixes_nothing() {
    let sandbox = Sandbox::new("doctor-fix-twice");
    fix(&sandbox, &[], 1);
    let before = snapshot(&sandbox);
    let document = fix(&sandbox, &[], 1);
    assert_eq!(document["fixed"], json!([]));
    assert_eq!(snapshot(&sandbox), before);
    assert!(!detail(&document, "config", "file").starts_with("created"));
}

#[test]
fn doctor_without_fix_has_no_fixed_array_and_creates_nothing() {
    let sandbox = Sandbox::new("doctor-fix-absent");
    let before = snapshot(&sandbox);
    let output = run(sandbox
        .vpt()
        .current_dir(sandbox.path())
        .args(["doctor", "-o", "json"]));
    let document = report(&output, 1);
    assert!(document.get("fixed").is_none(), "{document}");
    assert!(advice(&document, "config", "file").contains("vpt doctor --fix"));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn fix_with_named_checks_repairs_only_within_them() {
    let sandbox = configured("doctor-fix-named");
    let document = fix(&sandbox, &["config"], 0);
    assert_eq!(document["fixed"], json!([]));
    assert!(!sandbox.path().join("home/.vpt").exists());
    let document = fix(&sandbox, &["stores"], 0);
    assert_eq!(fixed(&document).len(), 9);
    assert!(sandbox.path().join("state/vpt").is_dir());
    let empty = Sandbox::new("doctor-fix-named-empty");
    let before = snapshot(&empty);
    let document = fix(&empty, &["stores"], 1);
    assert_eq!(document["fixed"], json!([]));
    assert_eq!(snapshot(&empty), before);
}

#[test]
fn a_failed_repair_stays_an_issue_with_its_reason_and_the_rest_continue() {
    let sandbox = configured("doctor-fix-failure");
    let parent = sandbox.path().join("home");
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o500)).unwrap();
    let document = fix(&sandbox, &[], 1);
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(fixed(&document), ["stores state_dir"]);
    assert_eq!(result(&document, "stores", "home"), "issue");
    assert!(
        detail(&document, "stores", "home").contains("could not create it: "),
        "{document}"
    );
    assert!(sandbox.path().join("state/vpt").is_dir());
    assert!(!sandbox.path().join("home/.vpt").exists());
}

#[test]
fn fix_creates_a_configured_home_link_and_the_stores_behind_it() {
    let sandbox = configured("doctor-fix-link");
    let target = sandbox.path().join("data/vpt");
    crate::observations_fixture::write_config(
        &sandbox,
        &format!(
            "symlink_target = {target:?}\n[source]\nrecordings_dir = {:?}\n",
            sandbox.path().join("voice-memos/Recordings")
        ),
    );
    let document = fix(&sandbox, &[], 0);
    let link = sandbox.path().join("home/.vpt");
    assert_eq!(std::fs::read_link(&link).unwrap(), target);
    assert_eq!(fixed(&document)[0], "stores home_link");
    assert_eq!(
        detail(&document, "stores", "home_link"),
        format!("created {} -> {}", link.display(), target.display())
    );
    assert!(!fixed(&document).contains(&"stores home".to_string()));
    assert!(target.join("audio").is_dir());
}
