use super::*;

#[test]
fn pre_rename_sync_failure_keeps_stage_and_target_absent() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-pre.m4a", b"staged");
    let target = audio.join("id.m4a");
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, _| Err(ArchiveError::Sync("file fault".into())),
        |_| panic!("directory sync must not run"),
    );
    assert_eq!(result, Err(ArchiveError::Sync("file fault".into())));
    assert_eq!(std::fs::read(&staged).expect("staged"), b"staged");
    assert_eq!(mode(&staged), 0o600);
    assert!(!target.exists());
}

#[test]
fn post_rename_directory_sync_failure_is_placed_unsynced() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-post.m4a", b"staged");
    let target = audio.join("id.m4a");
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, _| Ok(()),
        |_| {
            assert_eq!(std::fs::read(&target).expect("target"), b"staged");
            assert!(!staged.exists());
            Err(ArchiveError::Sync("directory fault".into()))
        },
    );
    assert_eq!(result, Err(ArchiveError::PlacedUnsynced(target.clone())));
    assert_eq!(std::fs::read(&target).expect("target"), b"staged");
    assert!(!staged.exists());
}

#[test]
fn syncing_a_missing_file_is_a_sync_error_and_an_existing_one_succeeds() {
    let (_temp, audio, root) = audio();
    let missing = sync_file_and_directory(&root, &audio.join("missing.m4a"), &digest(b"missing"));
    assert!(matches!(missing, Err(ArchiveError::Sync(_))), "{missing:?}");
    let present = write(&root, "present.m4a", b"present");
    assert_eq!(
        sync_file_and_directory(&root, &present, &digest(b"present")),
        Ok(())
    );
}

#[test]
fn duplicate_sync_revalidates_after_directory_sync() {
    let (_temp, audio, root) = audio();
    let archived = write(&root, "id.m4a", b"original");
    let saved = audio.with_extension("saved");
    let result = sync_file_and_directory_with(
        &root,
        &archived,
        &digest(b"original"),
        |_| Ok(()),
        |_| {
            std::fs::rename(&audio, &saved).expect("move root");
            std::fs::create_dir(&audio).expect("replacement");
            std::fs::write(&archived, b"replacement").expect("replacement file");
            Ok(())
        },
    );
    assert_eq!(result, Err(ArchiveError::Escape(archived.clone())));
    assert_eq!(
        std::fs::read(saved.join("id.m4a")).expect("old root"),
        b"original"
    );
    assert_eq!(
        std::fs::read(archived).expect("replacement"),
        b"replacement"
    );
}

#[test]
fn placed_outcome_revalidates_after_directory_sync() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-replace.m4a", b"original");
    let target = audio.join("id.m4a");
    let saved = audio.with_extension("saved");
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, _| Ok(()),
        |_| {
            std::fs::rename(&audio, &saved).expect("move root");
            std::fs::create_dir(&audio).expect("replacement");
            Ok(())
        },
    );
    assert_eq!(result, Err(ArchiveError::Escape(target.clone())));
    assert_eq!(
        std::fs::read(saved.join("id.m4a")).expect("old root"),
        b"original"
    );
    assert!(!staged.exists());
    assert!(!target.exists());
}

#[test]
fn duplicate_sync_refuses_content_changed_during_directory_sync() {
    let (_temp, _audio, root) = audio();
    let archived = write(&root, "id.m4a", b"original");
    let result =
        sync_file_and_directory_with(&root, &archived, &digest(b"original"), sync_open, |_| {
            std::fs::write(&archived, b"modified").unwrap();
            Ok(())
        });
    assert!(matches!(result, Err(ArchiveError::Sync(_))), "{result:?}");
    assert_eq!(std::fs::read(archived).unwrap(), b"modified");
}

#[test]
fn duplicate_sync_refuses_a_wrong_digest_before_syncing() {
    let (_temp, _audio, root) = audio();
    let archived = write(&root, "id.m4a", b"replacement");
    let result = sync_file_and_directory_with(
        &root,
        &archived,
        &digest(b"original"),
        |_| panic!("mismatching archive must not be synchronized"),
        |_| panic!("mismatching archive must not synchronize its directory"),
    );
    assert!(matches!(result, Err(ArchiveError::Sync(_))));
    assert_eq!(std::fs::read(archived).unwrap(), b"replacement");
}

#[test]
fn duplicate_sync_refuses_replacement_even_when_its_bytes_match() {
    let (_temp, audio, root) = audio();
    let archived = write(&root, "id.m4a", b"original");
    let moved = audio.join("moved.m4a");
    let result =
        sync_file_and_directory_with(&root, &archived, &digest(b"original"), sync_open, |_| {
            std::fs::rename(&archived, &moved).unwrap();
            std::fs::write(&archived, b"original").unwrap();
            Ok(())
        });
    assert!(matches!(result, Err(ArchiveError::Sync(_))));
    assert_eq!(std::fs::read(archived).unwrap(), b"original");
    assert_eq!(std::fs::read(moved).unwrap(), b"original");
}
