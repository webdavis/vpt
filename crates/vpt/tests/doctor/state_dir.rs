use crate::fixture::{check, configured, detail, run_json, snapshot};
use std::os::unix::fs::PermissionsExt;

#[test]
fn a_private_state_folder_passes_its_row() {
    let sandbox = configured("doctor-state-private");
    let state = sandbox.path().join("state/vpt");
    std::fs::create_dir(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
    let checks = run_json(&sandbox, 0);
    assert_eq!(check(&checks, "state_dir")["ok"], true);
    assert_eq!(detail(&checks, "state_dir"), state.display().to_string());
}

#[test]
fn an_absent_state_folder_passes_its_row_and_stays_absent() {
    let sandbox = configured("doctor-state-absent");
    let state = sandbox.path().join("state/vpt");
    let checks = run_json(&sandbox, 0);
    assert_eq!(check(&checks, "state_dir")["ok"], true);
    assert_eq!(
        detail(&checks, "state_dir"),
        format!("{} does not exist yet", state.display())
    );
    assert!(!state.exists());
}

#[test]
fn a_state_folder_with_the_wrong_mode_fails_its_row_with_the_fix_and_keeps_its_mode() {
    let sandbox = configured("doctor-state-mode");
    let state = sandbox.path().join("state/vpt");
    std::fs::create_dir(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o755)).unwrap();
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "state_dir")["ok"], false);
    assert_eq!(
        detail(&checks, "state_dir"),
        format!(
            "state directory '{0}' must have mode 0700; run: chmod 700 '{0}'",
            state.display()
        )
    );
    assert_eq!(snapshot(&sandbox), before);
}
