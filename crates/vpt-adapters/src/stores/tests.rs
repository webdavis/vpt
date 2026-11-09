use super::*;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::os::unix::fs::PermissionsExt;
mod publication;

pub(super) fn store() -> (tempfile::TempDir, PathBuf, FilesystemStores) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    std::fs::create_dir(&root).unwrap();
    let stores = FilesystemStores::open(std::slice::from_ref(&root)).unwrap();
    (temp, root, stores)
}

pub(super) fn entries(root: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn digest_of_matches_bytes_and_absence_is_none() {
    let (_temp, root, stores) = store();
    let target = root.join("a.md");
    std::fs::write(&target, b"hello").unwrap();
    let digest = stores.digest_of(&target).unwrap().unwrap();
    assert_eq!(
        digest.hex(),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    assert_eq!(digest, stores.digest_bytes(b"hello"));
    assert_eq!(stores.digest_of(&root.join("missing.md")), Ok(None));
}

#[test]
fn digest_open_hashes_from_start_even_when_cursor_is_nonzero() {
    let (_temp, root, _stores) = store();
    let target = root.join("a.md");
    std::fs::write(&target, b"hello").unwrap();
    let mut file = File::open(target).unwrap();
    file.seek(SeekFrom::Start(2)).unwrap();
    assert_eq!(
        digest_open(&mut file).unwrap().hex(),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn publish_replaces_target_with_mode_0644_and_no_temporary_leftover() {
    let (_temp, root, stores) = store();
    let target = root.join("a.md");
    std::fs::write(&target, b"old").unwrap();
    stores
        .publish(&target, Some(stores.digest_bytes(b"old")), b"new")
        .unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"new");
    assert_eq!(entries(&root), ["a.md"]);
    assert_eq!(
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o644
    );
    assert_eq!(stores.sync_directory_of(&target), Ok(()));
}

#[test]
fn existing_temporary_names_are_preserved_and_skipped() {
    let (_temp, root, stores) = store();
    let pid = std::process::id();
    for sequence in 0..2 {
        std::fs::write(
            root.join(format!(".a.md.vpt-{pid}-{sequence}")),
            b"abandoned",
        )
        .unwrap();
    }
    stores.publish(&root.join("a.md"), None, b"new").unwrap();
    assert_eq!(std::fs::read(root.join("a.md")).unwrap(), b"new");
    for sequence in 0..2 {
        assert_eq!(
            std::fs::read(root.join(format!(".a.md.vpt-{pid}-{sequence}"))).unwrap(),
            b"abandoned"
        );
    }
    assert!(!root.join(format!(".a.md.vpt-{pid}-2")).exists());
}

#[test]
fn directory_target_is_refused_before_staging_and_retry_can_publish_after_its_removal() {
    let (_temp, root, stores) = store();
    let target = root.join("a.md");
    std::fs::create_dir(&target).unwrap();
    assert_eq!(
        stores.publish(&target, None, b"first"),
        Err(StoreError::Escape(target.clone()))
    );
    assert_eq!(entries(&root), ["a.md"]);
    std::fs::remove_dir(&target).unwrap();
    stores.publish(&target, None, b"second").unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"second");
    assert_eq!(entries(&root), ["a.md"]);
}

#[test]
fn outside_nested_and_link_paths_are_refused() {
    let (temp, root, stores) = store();
    let outside = temp.path().join("outside.md");
    assert_eq!(
        stores.publish(&outside, None, b"x"),
        Err(StoreError::Escape(outside.clone()))
    );
    assert_eq!(
        stores.digest_of(&outside),
        Err(StoreError::Escape(outside.clone()))
    );
    assert_eq!(
        stores.sync_directory_of(&outside),
        Err(StoreError::Escape(outside))
    );

    let nested = root.join("sub/a.md");
    assert_eq!(
        stores.publish(&nested, None, b"x"),
        Err(StoreError::Escape(nested))
    );
    let real = root.join("real.md");
    let link = root.join("link.md");
    std::fs::write(&real, b"real").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(
        stores.digest_of(&link),
        Err(StoreError::Escape(link.clone()))
    );
    assert_eq!(
        stores.validate_path(&link),
        Err(StoreError::Escape(link.clone()))
    );
    assert_eq!(stores.validate_path(&real), Ok(()));
    assert_eq!(stores.validate_path(&root.join("absent.md")), Ok(()));
}

#[test]
fn direct_publish_refuses_symlink_target_before_creating_temporary() {
    let (_temp, root, stores) = store();
    let real = root.join("real.md");
    let link = root.join("link.md");
    std::fs::write(&real, b"real").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let before = entries(&root);
    assert_eq!(
        stores.publish(&link, None, b"new"),
        Err(StoreError::Escape(link.clone()))
    );
    assert_eq!(std::fs::read_link(&link).unwrap(), real);
    assert_eq!(std::fs::read(&real).unwrap(), b"real");
    assert_eq!(entries(&root), before);
}

#[test]
fn read_only_open_keeps_missing_root_absent_and_refuses_link() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing");
    let stores = FilesystemStores::open_read_only(std::slice::from_ref(&missing)).unwrap();
    assert!(stores.roots.is_empty());
    assert_eq!(stores.configured.as_slice(), std::slice::from_ref(&missing));
    assert!(!missing.exists());
    std::os::unix::fs::symlink(temp.path(), temp.path().join("link")).unwrap();
    assert!(FilesystemStores::open_read_only(&[temp.path().join("link")]).is_err());
}

#[test]
fn entries_lists_only_depth_one_files_with_full_paths_and_absent_roots() {
    let (temp, root, _stores) = store();
    std::fs::write(root.join("a.m4a"), b"abc").unwrap();
    std::fs::create_dir(root.join("child")).unwrap();
    std::fs::write(root.join("child/nested.m4a"), b"nested").unwrap();
    let absent = temp.path().join("absent");
    let stores = FilesystemStores::open_read_only(&[root.clone(), absent.clone()]).unwrap();
    let found = stores.entries(&root).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, root.join("a.m4a"));
    assert_eq!(found[0].size, 3);
    assert_eq!(stores.entries(&absent), Ok(vec![]));
    assert!(!absent.exists());
}

#[test]
fn entries_refuses_unconfigured_roots() {
    let (temp, _root, stores) = store();
    let unknown = temp.path().join("unknown");
    assert_eq!(stores.entries(&unknown), Err(StoreError::Escape(unknown)));
}

#[test]
fn entries_refuses_a_link_without_following_its_target() {
    let (_temp, root, stores) = store();
    let real = root.join("real.m4a");
    let link = root.join("linked.m4a");
    std::fs::write(&real, b"private audio").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(stores.entries(&root), Err(StoreError::Escape(link.clone())));
    assert_eq!(std::fs::read(&real).unwrap(), b"private audio");
    assert_eq!(std::fs::read_link(&link).unwrap(), real);
}

#[test]
fn entries_refuses_a_replaced_retained_root() {
    let (temp, root, stores) = store();
    let held = temp.path().join("held-store");
    std::fs::write(root.join("original.m4a"), b"original").unwrap();
    std::fs::rename(&root, &held).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("replacement.m4a"), b"replacement").unwrap();
    assert_eq!(stores.entries(&root), Err(StoreError::Escape(root.clone())));
    assert_eq!(
        std::fs::read(held.join("original.m4a")).unwrap(),
        b"original"
    );
    assert_eq!(
        std::fs::read(root.join("replacement.m4a")).unwrap(),
        b"replacement"
    );
}

#[test]
fn replaced_root_is_refused_for_digest_publish_and_sync() {
    let (temp, root, stores) = store();
    let old_root = temp.path().join("old-store");
    std::fs::write(root.join("a.md"), b"original").unwrap();
    std::fs::rename(&root, &old_root).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("a.md"), b"replacement").unwrap();
    let target = root.join("a.md");
    assert_eq!(
        stores.digest_of(&target),
        Err(StoreError::Escape(target.clone()))
    );
    assert_eq!(
        stores.publish(&target, None, b"new"),
        Err(StoreError::Escape(target.clone()))
    );
    assert_eq!(
        stores.sync_directory_of(&target),
        Err(StoreError::Escape(target))
    );
    assert_eq!(std::fs::read(root.join("a.md")).unwrap(), b"replacement");
    assert_eq!(std::fs::read(old_root.join("a.md")).unwrap(), b"original");
    assert_eq!(entries(&root), ["a.md"]);
    assert_eq!(entries(&old_root), ["a.md"]);
}

#[test]
fn private_output_roots_publish_mode_0600_without_changing_shared_directories() {
    let (_temp, root, stores) = store();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
    let stores = stores.with_private_roots(std::slice::from_ref(&root));
    let target = root.join("output.json");
    stores
        .publish(&target, None, b"private transcript")
        .unwrap();
    assert_eq!(
        std::fs::metadata(target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(root).unwrap().permissions().mode() & 0o777,
        0o755
    );
}
