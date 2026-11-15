use super::fixture::{configure, configured, detail, issues, result, run_json, snapshot};
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn portable_doctor_reports_that_no_vault_is_required() {
    let sandbox = configured("doctor-portable-vault");
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(
        detail(&document, "stores", "vault"),
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
    let document = run_json(&sandbox, 0);
    assert!(detail(&document, "stores", "vault").contains(vault.to_str().unwrap()));
    assert!(!document.to_string().contains("private-note-content-canary"));
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
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "stores", "vault"), "issue");
    assert_eq!(issues(&document), ["stores vault"]);
    assert!(detail(&document, "stores", "vault").contains("note.obsidian.vault_root"));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn doctor_accepts_roots_below_missing_folders_without_creating_them() {
    let sandbox = configured("doctor-missing-parents");
    let vault = sandbox.path().join("vault");
    fs::create_dir(&vault).unwrap();
    let nested = vault.join("vpt");
    configure(
        &sandbox,
        &format!(
            "[stores]\ntranscripts = {:?}\nanalysis = {:?}\nbriefs = {:?}\n",
            nested.join("transcripts"),
            nested.join("analysis"),
            nested.join("briefs")
        ),
    );
    let state = sandbox.path().join("home/.local/state/vpt");
    let config = fs::read_to_string(sandbox.config_path()).unwrap().replace(
        sandbox.path().join("state/vpt").to_str().unwrap(),
        state.to_str().unwrap(),
    );
    fs::write(sandbox.config_path(), config).unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 0);
    assert_eq!(issues(&document), Vec::<String>::new());
    assert_eq!(snapshot(&sandbox), before);
    assert!(!nested.exists());
    assert!(!sandbox.path().join("home/.local").exists());
}
