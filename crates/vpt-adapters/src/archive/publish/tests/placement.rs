use super::*;
use std::os::unix::fs::MetadataExt;

#[test]
fn publish_places_an_absent_target_and_removes_the_staged_name() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-1.m4a", b"bytes");
    let target = audio.join("id.m4a");
    assert_eq!(
        exclusive(&root, &staged, "id.m4a"),
        Ok(Published::Placed(target.clone()))
    );
    assert_eq!(std::fs::read(&target).expect("target"), b"bytes");
    assert_eq!(mode(&target), 0o600);
    assert!(!staged.exists());
}

#[test]
fn publish_never_replaces_an_existing_target_and_names_it() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-2.m4a", b"new");
    let target = write(&root, "id.m4a", b"old");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).expect("mode");
    assert_eq!(
        exclusive(&root, &staged, "id.m4a"),
        Ok(Published::Exists(audio.join("id.m4a")))
    );
    assert_eq!(std::fs::read(&target).expect("target"), b"old");
    assert_eq!(mode(&target), 0o640);
    assert_eq!(std::fs::read(&staged).expect("staged"), b"new");
    assert_eq!(mode(&staged), 0o600);
}

#[test]
fn the_archive_publishes_and_syncs_through_its_port() {
    let (_temp, audio, root) = audio();
    let archive = ClonefileArchive::open(&audio).expect("archive");
    let staged = write(&root, ".vpt-staging-4.m4a", b"bytes");
    let target = audio.join("id.m4a");
    let (verified, handle) = prepared(&root, &staged).unwrap();
    assert_eq!(
        archive.publish(&verified, &handle, "id.m4a"),
        Ok(Published::Placed(target.clone()))
    );
    assert_eq!(archive.sync_existing(&target, &digest(b"bytes")), Ok(()));
    assert_eq!(archive.archived().expect("list"), vec![target]);
}

#[test]
fn file_sync_precedes_placement_and_directory_sync_follows() {
    let (_temp, audio, root) = audio();
    let staged = write(&root, ".vpt-staging-order.m4a", b"bytes");
    let staged_inode = std::fs::metadata(&staged).expect("staged").ino();
    let target = audio.join("id.m4a");
    let calls = std::cell::RefCell::new(Vec::new());
    let result = exclusive_with(
        &root,
        &staged,
        "id.m4a",
        |_, file| {
            if file.metadata().expect("synced").ino() == staged_inode {
                assert!(!target.exists());
                calls.borrow_mut().push("staged");
            } else {
                assert_eq!(std::fs::read(&target).expect("target"), b"bytes");
                calls.borrow_mut().push("archive");
            }
            Ok(())
        },
        |_| {
            assert_eq!(std::fs::read(&target).expect("target"), b"bytes");
            assert!(staged.exists());
            calls.borrow_mut().push("directory");
            Ok(())
        },
    );
    assert_eq!(result, Ok(Published::Placed(target)));
    assert_eq!(*calls.borrow(), ["staged", "archive", "directory"]);
    assert!(!staged.exists());
}
