use crate::fixture::{advice, configure, configured, detail, issues, result, run_json, snapshot};

#[test]
fn bad_store_preserves_independent_checks() {
    let sandbox = configured("doctor-bad-root");
    configure(
        &sandbox,
        &format!(
            "[stores]\naudio = {:?}\n",
            sandbox.path().join("voice-memos/audio")
        ),
    );
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    assert_eq!(issues(&document), ["stores layout"]);
    assert_eq!(result(&document, "leftovers", "staged_audio"), "not_run");
    assert_eq!(
        detail(&document, "leftovers", "staged_audio"),
        "the store folders did not resolve"
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
                sandbox.path().join("absent/../released")
            ),
        );
    }
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, if bad_store { 1 } else { 0 });
    for key in ["audio", "drafts", "engine_outputs", "transcripts"] {
        assert!(detail(&document, "stores", key).contains(repository.to_str().unwrap()));
    }
    assert_eq!(
        advice(&document, "stores", "audio"),
        "confirm the ignore rules exclude audio"
    );
    for key in ["audio", "drafts", "engine_outputs"] {
        assert_eq!(result(&document, "stores", key), "warning", "{key}");
    }
    assert!(advice(&document, "stores", "drafts").contains("sensitive"));
    assert!(advice(&document, "stores", "engine_outputs").contains("sensitive"));
    assert_eq!(result(&document, "stores", "transcripts"), "ok");
    assert_eq!(advice(&document, "stores", "transcripts"), "");
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
fn one_bad_apple_subdirectory_preserves_other_checks() {
    let sandbox = configured("doctor-bad-subdir");
    std::os::unix::fs::symlink(
        sandbox.path(),
        sandbox.path().join("voice-memos/Recordings/Capture"),
    )
    .unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    assert_eq!(issues(&document), ["recordings Capture"]);
    assert_eq!(result(&document, "recordings", "CaptureRecovery"), "ok");
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
    let document = run_json(&sandbox, 0);
    for name in vpt_adapters::APPLE_SUBDIRECTORIES {
        assert_eq!(detail(&document, "recordings", name), "1 entries");
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
    let document = run_json(&sandbox, if correct { 0 } else { 1 });
    let expected = if correct { "ok" } else { "issue" };
    assert_eq!(result(&document, "stores", "home_link"), expected);
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
    let document = run_json(&sandbox, 0);
    assert!(detail(&document, "stores", "home_link").starts_with("followed, not managed:"));
}

#[test]
fn staged_files_are_reported_without_cleanup() {
    let sandbox = configured("doctor-staged");
    let audio = sandbox.path().join("home/.vpt/audio");
    std::fs::create_dir_all(&audio).unwrap();
    std::fs::write(audio.join(".vpt-staging-owned"), b"private-audio-canary").unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    assert_eq!(result(&document, "leftovers", "staged_audio"), "issue");
    assert!(detail(&document, "leftovers", "staged_audio").contains(".vpt-staging-owned"));
    assert!(!document.to_string().contains("private-audio-canary"));
    assert_eq!(snapshot(&sandbox), before);
}

#[test]
fn versions_kept_beside_notes_are_listed_by_name() {
    let sandbox = configured("doctor-kept");
    let transcripts = sandbox.path().join("home/.vpt/transcripts");
    std::fs::create_dir_all(&transcripts).unwrap();
    std::fs::write(
        transcripts.join(".note.md.vpt-kept-41-2"),
        b"private-note-canary",
    )
    .unwrap();
    std::fs::write(transcripts.join(".note.md.vpt-41-3"), b"staged").unwrap();
    let before = snapshot(&sandbox);
    let document = run_json(&sandbox, 1);
    assert_eq!(issues(&document), ["leftovers kept_edits"]);
    assert_eq!(
        detail(&document, "leftovers", "kept_edits"),
        "1 version vpt kept beside notes: transcripts/.note.md.vpt-kept-41-2"
    );
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
    crate::support::fake_engine()
}
