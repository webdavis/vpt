use super::*;

#[test]
fn archive_lists_only_regular_archived_and_staging_files() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let archive_name = "2026-08-24T144736-4f3ab19c02de.m4a";
    let staging_name = format!("{STAGING_PREFIX}123.m4a");
    std::fs::write(audio.join(archive_name), b"archive").expect("archive");
    std::fs::write(audio.join(&staging_name), b"staging").expect("staging");
    std::fs::write(audio.join("notes.txt"), b"note").expect("note");
    std::os::unix::fs::symlink(audio.join(archive_name), audio.join("linked.m4a")).expect("link");
    assert_eq!(
        archive.archived().expect("archives"),
        vec![audio.join(archive_name)]
    );
    assert_eq!(
        archive.staged_leftovers().expect("staging"),
        vec![audio.join(staging_name)]
    );
    assert_eq!(archive.target("id.m4a"), audio.join("id.m4a"));
}

#[test]
fn absent_read_only_archive_stays_absent_and_refuses_stage_before_clone() {
    let temp = crate::test_temp::tempdir().expect("temp");
    let audio = temp
        .path()
        .canonicalize()
        .expect("canonical")
        .join("missing");
    let archive = ClonefileArchive::open_read_only(&audio).expect("read only");
    assert!(archive.archived().expect("archives").is_empty());
    assert!(archive.staged_leftovers().expect("staging").is_empty());
    let failure = archive
        .stage(|_, _, _| panic!("clone must not run"))
        .expect_err("absent");
    assert!(matches!(failure.cause, ArchiveError::Io(_)));
    assert_eq!(failure.owned_staging, None);
    assert!(!audio.exists());
}

#[test]
fn open_digest_and_cleanup_validation_refuse_links_and_outside_paths() {
    let (temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let real = audio.join("real.m4a");
    let link = audio.join("link.m4a");
    std::fs::write(&real, b"bytes").expect("real");
    std::os::unix::fs::symlink(&real, &link).expect("link");
    assert_eq!(
        archive.open(&link).err(),
        Some(ArchiveError::Escape(link.clone()))
    );
    assert_eq!(
        archive.digest(&link),
        Err(ArchiveError::Escape(link.clone()))
    );
    assert_eq!(
        archive.validate_trash_path(&link),
        Err(ArchiveError::Escape(link))
    );
    let outside = temp
        .path()
        .canonicalize()
        .expect("canonical")
        .join("Recordings/source.m4a");
    assert_eq!(archive.digest(&outside), Err(ArchiveError::Escape(outside)));
    assert!(matches!(
        archive.open(&audio.join("missing.m4a")),
        Err(ArchiveError::Io(_))
    ));
}

#[test]
fn root_replacement_during_stage_returns_no_cleanup_path() {
    let (_temp, audio, store, handle) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let saved = audio.with_extension("saved");
    let failure = archive
        .stage(|directory, identity, name| {
            let kind = store.clone_into(&handle, directory, identity, name)?;
            std::fs::rename(&audio, &saved).expect("move root");
            std::fs::create_dir(&audio).expect("replace root");
            std::fs::write(audio.join(name), b"replacement").expect("replacement file");
            Ok(kind)
        })
        .expect_err("replaced");
    assert_eq!(failure.cause, ArchiveError::Escape(audio.clone()));
    assert_eq!(failure.owned_staging, None);
    assert_eq!(std::fs::read_dir(&saved).expect("original").count(), 1);
    assert_eq!(std::fs::read_dir(&audio).expect("replacement").count(), 1);
}

#[test]
fn cleanup_validation_refuses_replaced_root_without_touching_replacement() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let saved = audio.with_extension("saved");
    std::fs::rename(&audio, &saved).expect("move root");
    std::fs::create_dir(&audio).expect("replacement");
    let path = audio.join(".vpt-staging-test.m4a");
    std::fs::write(&path, b"replacement").expect("file");
    assert_eq!(
        archive.validate_trash_path(&path),
        Err(ArchiveError::Escape(audio))
    );
    assert_eq!(std::fs::read(path).expect("bytes"), b"replacement");
}

#[test]
fn listing_refuses_a_replaced_root() {
    let (_temp, audio, _, _) = fixture(b"source");
    let archive = ClonefileArchive::open(&audio).expect("archive");
    std::fs::rename(&audio, audio.with_extension("saved")).expect("move root");
    std::fs::create_dir(&audio).expect("replacement");
    std::fs::write(audio.join("new.m4a"), b"replacement").expect("file");
    assert_eq!(archive.archived(), Err(ArchiveError::Escape(audio.clone())));
    assert_eq!(archive.staged_leftovers(), Err(ArchiveError::Escape(audio)));
}
