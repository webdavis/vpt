use super::*;

fn configured_tags(name: &str) -> (Sandbox, PathBuf, PathBuf) {
    let (sandbox, terms) = configured(name);
    let tags = sandbox.config_path().with_file_name("known-tags.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path={terms:?}\n[tags]\nknown_tags_path={tags:?}\n"),
    );
    (sandbox, terms, tags)
}

#[test]
fn startup_recovers_tag_vocabulary_before_new_term_work() {
    let (sandbox, terms, tags) = configured_tags("recover-tags");
    pending_vocabulary(&sandbox, &tags, "Private tag", VocabularyKind::Tags);
    success(&invoke(&sandbox, "Private term"));
    assert_eq!(fs::read_to_string(&tags).unwrap(), "private-tag\n");
    assert_eq!(fs::read_to_string(&terms).unwrap(), "Private term\n");
    assert_eq!(
        fs::metadata(&tags).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        ledger(&sandbox).vocabulary_publication(&tags).unwrap(),
        None
    );
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
    let events = fs::read_to_string(sandbox.path().join("state/vpt/events.jsonl")).unwrap();
    assert!(!events.contains("private-tag"));
    assert!(!events.contains("Private term"));
}

#[test]
fn tag_recovery_acknowledges_already_published_bytes_without_rewriting() {
    let (sandbox, _, tags) = configured_tags("recover-tags-ack");
    let saved = pending_vocabulary(&sandbox, &tags, "Private tag", VocabularyKind::Tags);
    vpt_adapters::known_terms::Update::restore(&saved.entry, saved.bytes)
        .unwrap()
        .publish()
        .unwrap();
    let modified = fs::metadata(&tags).unwrap().modified().unwrap();
    success(&invoke(&sandbox, "term"));
    assert_eq!(fs::metadata(&tags).unwrap().modified().unwrap(), modified);
    assert!(ledger(&sandbox).pending_publications().unwrap().is_empty());
}

#[test]
fn tag_recovery_uses_tag_configuration_and_refuses_relocation() {
    let (sandbox, terms, tags) = configured_tags("recover-tags-moved");
    let saved = pending_vocabulary(&sandbox, &tags, "Private tag", VocabularyKind::Tags);
    let changed = tags.with_file_name("other-tags.txt");
    fixture::write_config(
        &sandbox,
        &format!("[reconcile]\nknown_terms_path={terms:?}\n[tags]\nknown_tags_path={changed:?}\n"),
    );
    let output = invoke(&sandbox, "term");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(stderr(&output).contains("known_tags_path_changed"));
    assert!(!tags.exists());
    assert!(!changed.exists());
    assert!(!terms.exists());
    assert_eq!(
        ledger(&sandbox).vocabulary_publication(&tags).unwrap(),
        Some(saved)
    );
}

#[test]
fn tag_recovery_preserves_operator_edits_and_pending_bytes() {
    let (sandbox, terms, tags) = configured_tags("recover-tags-edited");
    let saved = pending_vocabulary(&sandbox, &tags, "Private tag", VocabularyKind::Tags);
    fs::write(&tags, "operator\n").unwrap();
    let output = invoke(&sandbox, "term");
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(stderr(&output).contains("publication_target_modified"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(&tags).unwrap(), "operator\n");
    assert!(!terms.exists());
    assert_eq!(
        ledger(&sandbox).vocabulary_publication(&tags).unwrap(),
        Some(saved)
    );
}
