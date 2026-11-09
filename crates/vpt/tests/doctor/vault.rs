use super::fixture::{check, configure, configured, detail, run_json, snapshot};
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn portable_doctor_reports_that_no_vault_is_required() {
    let sandbox = configured("doctor-portable-vault");
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 0);
    assert_eq!(
        detail(&checks, "vault"),
        "not required by the portable profile"
    );
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn obsidian_doctor_checks_the_configured_vault_without_writes() {
    let sandbox = configured("doctor-obsidian-vault");
    let vault = sandbox.path().join("vault");
    fs::create_dir(&vault).unwrap();
    fs::set_permissions(&vault, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(vault.join("private-note.md"), "private-note-content-canary").unwrap();
    configure(
        &sandbox,
        &format!("[note]\nprofile = 'obsidian'\n[note.obsidian]\nvault_root = {vault:?}\n"),
    );
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 0);
    assert!(detail(&checks, "vault").contains(vault.to_str().unwrap()));
    assert!(!format!("{checks:?}").contains("private-note-content-canary"));
    assert_eq!(snapshot(&sandbox), before);
    assert_eq!(
        fs::metadata(vault).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn missing_vault_is_reported_without_preventing_independent_checks() {
    let sandbox = configured("doctor-missing-vault");
    let vault = sandbox.path().join("missing-vault");
    configure(
        &sandbox,
        &format!("[note]\nprofile = 'obsidian'\n[note.obsidian]\nvault_root = {vault:?}\n"),
    );
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "vault")["ok"], false);
    assert_eq!(check(&checks, "helper")["ok"], true);
    assert_eq!(check(&checks, "recordings_dir")["ok"], true);
    assert!(detail(&checks, "vault").contains("note.obsidian.vault_root"));
    assert_eq!(snapshot(&sandbox), before);
}
