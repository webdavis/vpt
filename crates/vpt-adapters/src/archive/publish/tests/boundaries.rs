use super::*;

#[test]
fn a_staged_name_outside_the_root_or_a_nested_target_is_an_escape() {
    let (temp, audio, root) = audio();
    let outside = temp
        .path()
        .canonicalize()
        .expect("canonical")
        .join("elsewhere.m4a");
    std::fs::write(&outside, b"outside").expect("outside");
    assert_eq!(
        exclusive(&root, &outside, "id.m4a"),
        Err(ArchiveError::Escape(outside.clone()))
    );
    let staged = write(&root, ".vpt-staging-3.m4a", b"staged");
    assert_eq!(
        exclusive(&root, &staged, "sub/id.m4a"),
        Err(ArchiveError::Escape(audio.join("sub/id.m4a")))
    );
    assert_eq!(std::fs::read(outside).expect("outside"), b"outside");
    assert_eq!(std::fs::read(staged).expect("staged"), b"staged");
    assert!(!audio.join("id.m4a").exists());
}

#[test]
fn publication_refuses_linked_stage_and_target_without_touching_referents() {
    let (temp, audio, root) = audio();
    let outside = temp
        .path()
        .canonicalize()
        .expect("canonical")
        .join("outside.m4a");
    std::fs::write(&outside, b"outside").expect("outside");
    let outside_mode = mode(&outside);
    let staged_link = audio.join(".vpt-staging-link.m4a");
    std::os::unix::fs::symlink(&outside, &staged_link).expect("staged link");
    assert_eq!(
        exclusive(&root, &staged_link, "id.m4a"),
        Err(ArchiveError::Escape(staged_link.clone()))
    );
    let staged = write(&root, ".vpt-staging-real.m4a", b"staged");
    let target_link = audio.join("linked.m4a");
    std::os::unix::fs::symlink(&outside, &target_link).expect("target link");
    assert_eq!(
        exclusive(&root, &staged, "linked.m4a"),
        Err(ArchiveError::Escape(target_link))
    );
    assert_eq!(std::fs::read(&outside).expect("outside"), b"outside");
    assert_eq!(mode(&outside), outside_mode);
    assert_eq!(std::fs::read(staged).expect("staged"), b"staged");
}

#[test]
fn publication_refuses_a_replaced_root_before_returning_either_outcome() {
    for already_exists in [false, true] {
        let (_temp, audio, root) = audio();
        let staged = write(&root, ".vpt-staging-race.m4a", b"staged");
        if already_exists {
            write(&root, "id.m4a", b"original");
        }
        let (verified, handle) = prepared(&root, &staged).unwrap();
        let saved = audio.with_extension("saved");
        std::fs::rename(&audio, &saved).expect("move root");
        std::fs::create_dir(&audio).expect("replacement");
        assert_eq!(
            super::super::exclusive(&root, &verified, &handle, "id.m4a"),
            Err(ArchiveError::Escape(audio.join("id.m4a")))
        );
        assert_eq!(
            std::fs::read(saved.join(".vpt-staging-race.m4a")).expect("staged"),
            b"staged"
        );
        assert!(
            std::fs::read_dir(&audio)
                .expect("replacement")
                .next()
                .is_none()
        );
    }
}

#[test]
fn exists_outcome_revalidates_after_root_replacement() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-exists.m4a", b"staged");
    write(&root, "id.m4a", b"original");
    let saved = audio.with_extension("saved");
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, _| {
            std::fs::rename(&audio, &saved).expect("move root");
            std::fs::create_dir(&audio).expect("replacement");
            Ok(())
        },
        |_| panic!("directory sync must not run for a duplicate"),
    );
    assert_eq!(result, Err(ArchiveError::Escape(staged)));
    assert_eq!(
        std::fs::read(saved.join("id.m4a")).expect("old target"),
        b"original"
    );
    assert_eq!(
        std::fs::read(saved.join(".vpt-staging-exists.m4a")).expect("old stage"),
        b"staged"
    );
    assert!(
        std::fs::read_dir(audio)
            .expect("replacement")
            .next()
            .is_none()
    );
}

#[test]
fn duplicate_sync_refuses_links_and_outside_paths() {
    let (temp, audio, root) = audio();
    let outside = temp
        .path()
        .canonicalize()
        .expect("canonical")
        .join("outside.m4a");
    std::fs::write(&outside, b"outside").expect("outside");
    let link = audio.join("link.m4a");
    std::os::unix::fs::symlink(&outside, &link).expect("link");
    assert_eq!(
        sync_file_and_directory(&root, &outside, &digest(b"outside")),
        Err(ArchiveError::Escape(outside.clone()))
    );
    assert_eq!(
        sync_file_and_directory(&root, &link, &digest(b"outside")),
        Err(ArchiveError::Escape(link))
    );
    let archive = ClonefileArchive::open(&audio).expect("archive");
    assert_eq!(
        archive.sync_existing(&outside, &digest(b"outside")),
        Err(ArchiveError::Escape(outside))
    );
}

#[test]
fn publication_refuses_stage_changes_during_file_sync() {
    for replace in [false, true] {
        let (_temp, audio, root) = audio();
        let staged = write(&root, ".vpt-staging-sync.m4a", b"original");
        let held = audio.join("held.m4a");
        let result = exclusive_with(
            &root,
            &staged,
            "id.m4a",
            |_, _| {
                if replace {
                    std::fs::rename(&staged, &held).unwrap();
                }
                std::fs::write(&staged, b"modified").unwrap();
                Ok(())
            },
            |_| panic!("changed staging must not be published"),
        );
        assert!(result.is_err(), "{result:?}");
        assert!(!audio.join("id.m4a").exists());
        assert_eq!(std::fs::read(&staged).unwrap(), b"modified");
        if replace {
            assert_eq!(std::fs::read(&held).unwrap(), b"original");
        }
    }
}

#[test]
fn publication_refuses_archive_changes_during_directory_sync() {
    for replace in [false, true] {
        let (_temp, audio, root) = audio();
        let staged = write(&root, ".vpt-staging-sync.m4a", b"original");
        let target = audio.join("id.m4a");
        let held = audio.join("held.m4a");
        let result = exclusive_with(
            &root,
            &staged,
            "id.m4a",
            |_, _| Ok(()),
            |_| {
                if replace {
                    std::fs::rename(&target, &held).unwrap();
                }
                std::fs::write(&target, b"modified").unwrap();
                Ok(())
            },
        );
        assert!(result.is_err(), "{result:?}");
        assert_eq!(std::fs::read(&target).unwrap(), b"modified");
        if replace {
            assert_eq!(std::fs::read(&held).unwrap(), b"original");
        }
    }
}
