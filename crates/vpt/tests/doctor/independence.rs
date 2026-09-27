use crate::fixture::{check, configure, configured, detail, run_json, snapshot};

#[test]
fn bad_store_preserves_independent_checks() {
    let sandbox = configured("doctor-bad-root");
    configure(
        &sandbox,
        &format!(
            "[stores]\naudio = {:?}\n",
            sandbox.path().join("absent/audio")
        ),
    );
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "stores")["ok"], false);
    for name in [
        "helper",
        "recordings_dir",
        "git_tree:audio",
        "symlink",
        "source_gone",
    ] {
        assert_eq!(check(&checks, name)["ok"], true, "{name}");
    }
    assert_eq!(
        detail(&checks, "cleanup_pending"),
        "not run: store roots did not resolve"
    );
    assert_eq!(snapshot(&sandbox), before);
}

fn linked_home(bad_store: bool) {
    let sandbox = configured("doctor-git-home");
    let repository = sandbox.path().join("repository");
    std::fs::create_dir_all(repository.join(".git")).unwrap();
    std::fs::create_dir(repository.join("vault")).unwrap();
    std::os::unix::fs::symlink(repository.join("vault"), sandbox.path().join("home/.vpt")).unwrap();
    if bad_store {
        configure(
            &sandbox,
            &format!(
                "[stores]\nreleased = {:?}\n",
                sandbox.path().join("absent/released")
            ),
        );
    }
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, if bad_store { 3 } else { 0 });
    for key in ["audio", "drafts", "engine_outputs", "transcripts"] {
        let name = format!("git_tree:{key}");
        assert!(detail(&checks, &name).contains(repository.to_str().unwrap()));
    }
    assert!(detail(&checks, "git_tree:audio").contains("confirm the ignore rules exclude audio"));
    assert!(detail(&checks, "git_tree:drafts").contains("sensitive"));
    assert!(detail(&checks, "git_tree:engine_outputs").contains("sensitive"));
    assert!(!detail(&checks, "git_tree:transcripts").contains("sensitive"));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn resolved_home_link_preserves_git_checks() {
    linked_home(false);
}
#[test]
fn resolved_home_link_preserves_git_checks_with_bad_store() {
    linked_home(true);
}

#[test]
fn unavailable_source_marks_only_its_dependencies() {
    let sandbox = configured("doctor-missing-source");
    std::fs::rename(
        sandbox.path().join("voice-memos/Recordings"),
        sandbox.path().join("held-source"),
    )
    .unwrap();
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "recordings_dir")["ok"], false);
    for name in vpt_adapters::APPLE_SUBDIRECTORIES {
        assert_eq!(check(&checks, &format!("subdirectory:{name}"))["ok"], false);
        assert_eq!(
            detail(&checks, &format!("subdirectory:{name}")),
            "not run: recordings_dir could not be opened"
        );
    }
    assert_eq!(check(&checks, "helper")["ok"], true);
    assert_eq!(check(&checks, "source_gone")["ok"], true);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn one_bad_apple_subdirectory_preserves_other_checks() {
    let sandbox = configured("doctor-bad-subdir");
    std::os::unix::fs::symlink(
        sandbox.path(),
        sandbox.path().join("voice-memos/Recordings/Capture"),
    )
    .unwrap();
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "subdirectory:Capture")["ok"], false);
    assert_eq!(check(&checks, "subdirectory:CaptureRecovery")["ok"], true);
    assert_eq!(check(&checks, "helper")["ok"], true);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn apple_subdirectories_report_real_counts() {
    let sandbox = configured("doctor-counts");
    for name in vpt_adapters::APPLE_SUBDIRECTORIES {
        let directory = sandbox.path().join("voice-memos/Recordings").join(name);
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("owned"), b"fixture").unwrap();
    }
    let checks = run_json(&sandbox, 0);
    for name in vpt_adapters::APPLE_SUBDIRECTORIES {
        assert_eq!(
            detail(&checks, &format!("subdirectory:{name}")),
            "1 entries"
        );
    }
}

fn managed_link(correct: bool) {
    let sandbox = configured("doctor-managed-link");
    let target = sandbox.path().join("target");
    let other = sandbox.path().join("other");
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::os::unix::fs::symlink(
        if correct { &target } else { &other },
        sandbox.path().join("home/.vpt"),
    )
    .unwrap();
    let config = format!(
        "config_version = 1\n[home]\nstate_dir = {:?}\nsymlink_target = {target:?}\n[source]\nrecordings_dir = {:?}\n",
        sandbox.path().join("state/vpt"),
        sandbox.path().join("voice-memos/Recordings")
    );
    std::fs::write(sandbox.config_path(), config).unwrap();
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, if correct { 0 } else { 3 });
    assert_eq!(check(&checks, "symlink")["ok"], correct);
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn managed_symlink_is_verified_without_repair() {
    managed_link(true);
}
#[test]
fn wrong_managed_symlink_remains_unchanged() {
    managed_link(false);
}

#[test]
fn unmanaged_home_link_is_reported() {
    let sandbox = configured("doctor-unmanaged-link");
    let target = sandbox.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::os::unix::fs::symlink(&target, sandbox.path().join("home/.vpt")).unwrap();
    let checks = run_json(&sandbox, 0);
    assert!(detail(&checks, "symlink").starts_with("followed, not managed:"));
}

#[test]
fn staged_files_are_reported_without_cleanup() {
    let sandbox = configured("doctor-staged");
    let audio = sandbox.path().join("home/.vpt/audio");
    std::fs::create_dir_all(&audio).unwrap();
    std::fs::write(audio.join(".vpt-staging-owned"), b"private-audio-canary").unwrap();
    let before = snapshot(&sandbox);
    let checks = run_json(&sandbox, 3);
    assert_eq!(check(&checks, "cleanup_pending")["ok"], false);
    assert!(detail(&checks, "cleanup_pending").contains(".vpt-staging-owned"));
    assert!(!detail(&checks, "cleanup_pending").contains("private-audio-canary"));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn doctor_does_not_refresh_titles_or_run_hooks() {
    let sandbox = configured("doctor-observation-only");
    let title = sandbox.path().join("voice-memos/CloudRecordings.db");
    std::fs::write(title, b"unreadable-title-canary").unwrap();
    configure(
        &sandbox,
        &format!(
            "[report]\ncommand = [{:?}, \"command-sink\"]\n[alert]\ncommand = [{:?}, \"command-sink\"]\n",
            support_engine(),
            support_engine()
        ),
    );
    let before = snapshot(&sandbox);
    run_json(&sandbox, 0);
    assert_eq!(snapshot(&sandbox), before);
    assert!(!sandbox.fake_log().exists());
}

fn support_engine() -> &'static str {
    crate::support::FAKE_ENGINE
}
