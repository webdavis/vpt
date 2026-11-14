use super::*;
use std::os::unix::fs::MetadataExt;

#[test]
fn a_failed_archive_sync_is_placed_unsynced_and_keeps_the_stage() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-sync.m4a", b"verified");
    let staged_inode = std::fs::metadata(&staged).expect("staged").ino();
    let target = audio.join("id.m4a");
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, file| {
            if file.metadata().expect("synced").ino() == staged_inode {
                Ok(())
            } else {
                Err(ArchiveError::Sync("archive fault".into()))
            }
        },
        |_| panic!("directory sync must not run"),
    );
    assert_eq!(result, Err(ArchiveError::PlacedUnsynced(target.clone())));
    assert_eq!(std::fs::read(&target).expect("target"), b"verified");
    assert_eq!(std::fs::read(&staged).expect("staged"), b"verified");
}

#[test]
fn a_failed_final_check_is_changed_and_keeps_the_stage() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-check.m4a", b"verified");
    let target = audio.join("id.m4a");
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, _| Ok(()),
        |_| {
            std::fs::write(&target, b"rewritten").expect("rewrite");
            Ok(())
        },
    );
    assert_eq!(result, Err(ArchiveError::Changed(target.clone())));
    assert_eq!(std::fs::read(&staged).expect("staged"), b"verified");
}

#[test]
fn an_unreadable_archive_after_placement_is_placed_unsynced_and_keeps_the_stage() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-unread.m4a", b"verified");
    let target = audio.join("id.m4a");
    let result = exclusive_placing(&root, &staged, "id.m4a", |root, _, _, _| {
        write(root, "id.m4a", b"verified");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o000)).expect("mode");
        Ok(Placement::Cloned)
    });
    assert_eq!(result, Err(ArchiveError::PlacedUnsynced(target.clone())));
    assert_eq!(std::fs::read(&staged).expect("staged"), b"verified");
}

#[test]
fn a_staging_file_that_cannot_be_removed_still_publishes_the_archive() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-kept.m4a", b"verified");
    let target = audio.join("id.m4a");
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, _| Ok(()),
        |_| {
            std::fs::hard_link(&staged, audio.join("second-name")).expect("link");
            Ok(())
        },
    );
    assert_eq!(result, Ok(Published::PlacedLeavingStage(target.clone())));
    assert_eq!(std::fs::read(&target).expect("target"), b"verified");
    assert_eq!(std::fs::read(&staged).expect("staged"), b"verified");
}
