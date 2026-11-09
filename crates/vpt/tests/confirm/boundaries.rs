use super::*;

#[test]
fn tag_list_cannot_alias_configuration_or_the_term_list() {
    for index in 0..3 {
        let (sandbox, terms) = configured("confirm-tag-alias");
        let tags = sandbox.config_path().with_file_name("known-tags.txt");
        fixture::write_config(
            &sandbox,
            &format!("[reconcile]\nknown_terms_path={terms:?}\n[tags]\nknown_tags_path={tags:?}\n"),
        );
        match index {
            0 => fs::hard_link(sandbox.config_path(), &tags).unwrap(),
            1 => {
                fs::write(&terms, "original\n").unwrap();
                fs::hard_link(&terms, &tags).unwrap();
            }
            _ => {
                fs::write(&terms, "original\n").unwrap();
                std::os::unix::fs::symlink(&terms, &tags).unwrap();
            }
        }
        let before = fixture::snapshot(&sandbox);
        let output = invoke(&sandbox, "Private canary");
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(stderr(&output).contains("tags.known_tags_path"));
        assert!(!stderr(&output).contains("Private canary"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn a_second_name_for_the_configuration_is_refused_before_mutation() {
    let (sandbox, path) = configured("confirm-config-alias");
    fs::hard_link(sandbox.config_path(), &path).unwrap();
    let before = fixture::snapshot(&sandbox);
    let output = invoke(&sandbox, "Private canary");
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(fixture::snapshot(&sandbox), before);
}

#[test]
fn protected_destinations_refuse_before_creating_state_or_changing_files() {
    for index in 0..4 {
        let (sandbox, _) = configured("confirm-protected");
        let source = sandbox.path().join("voice-memos/Recordings");
        let path = match index {
            0 => sandbox.config_path(),
            1 => sandbox.path().join("state/vpt/terms.txt"),
            2 => source.join("terms.txt"),
            _ => sandbox.path().join("home/.vpt/released/terms.txt"),
        };
        fixture::write_config(
            &sandbox,
            &format!(
                "[source]\nrecordings_dir={source:?}\n[reconcile]\nknown_terms_path={path:?}\n"
            ),
        );
        let before = fixture::snapshot(&sandbox);
        let output = invoke(&sandbox, "Private canary");
        assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
        assert!(stderr(&output).contains("reconcile.known_terms_path"));
        assert!(!stderr(&output).contains("Private canary"));
        assert_eq!(fixture::snapshot(&sandbox), before);
    }
}

#[test]
fn a_redirected_list_does_not_change_its_target() {
    let (sandbox, path) = configured("confirm-symlink");
    let outside = sandbox.path().join("outside.txt");
    fs::write(&outside, "Original\n").unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    let output = invoke(&sandbox, "Private canary");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(&outside).unwrap(), "Original\n");
    assert!(
        fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn an_export_failure_preserves_confirmation_and_reports_a_safe_warning() {
    let (sandbox, path) = configured("confirm-log-failure");
    fixture::seed(&sandbox, vec![]);
    fs::create_dir(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    let output = invoke(&sandbox, "Private canary");
    let result = success(&output);
    assert!(!result["warnings"].as_array().unwrap().is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), "Private canary\n");
    assert!(!stdout(&output).contains("Private canary"));
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn option_like_term_text_remains_data() {
    let (sandbox, path) = configured("confirm-option-text");
    success(&invoke(&sandbox, "--json"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "--json\n");
}

#[test]
fn dry_run_leaves_pending_terms_untouched_and_retention_startup_recovers_them() {
    let (sandbox, path) = configured("confirm-retention-startup");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path={path:?}\n[retention]\nenabled=true\n"),
    );
    let saved = pending(&sandbox, &path, "Recovered");
    let before = fixture::snapshot(&sandbox);
    let output = support::run_with_budget(
        sandbox
            .vpt()
            .args(["retention", "run", "--dry-run", "--json"]),
        std::time::Duration::from_secs(5),
    );
    success(&output);
    assert_eq!(fixture::snapshot(&sandbox), before);
    assert!(!path.exists());
    assert_eq!(
        ledger(&sandbox).vocabulary_publication(&path).unwrap(),
        Some(saved)
    );
    let output = support::run_with_budget(
        sandbox.vpt().args(["retention", "run", "--json"]),
        std::time::Duration::from_secs(5),
    );
    success(&output);
    assert_eq!(fs::read_to_string(&path).unwrap(), "Recovered\n");
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn human_output_and_custom_list_parent_work_without_changing_shared_permissions() {
    let (sandbox, _) = configured("confirm-human");
    let shared = sandbox.path().join("shared");
    fs::create_dir(&shared).unwrap();
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o755)).unwrap();
    let path = shared.join("private/known-terms.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path={path:?}\n"),
    );
    let output = support::run_with_budget(
        sandbox.vpt().args(["confirm", "--term", "Private canary"]),
        std::time::Duration::from_secs(5),
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("Confirmed term"));
    assert!(!stdout(&output).contains("Private canary"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "Private canary\n");
    assert_eq!(
        fs::metadata(&shared).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(
        fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}
