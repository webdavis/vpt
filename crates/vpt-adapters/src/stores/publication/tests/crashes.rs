use super::recovery::pending;
use super::*;
use vpt_application::repair_publication;

const ASIDE: RenameSupport = RenameSupport {
    swap: false,
    exclusive: true,
    hard_links: true,
};

fn crash_at(support: RenameSupport, crash: fn(Checkpoint) -> bool) -> (tempfile::TempDir, PathBuf) {
    let (temp, root, stores) = store();
    let target = root.join("note.md");
    fs::write(&target, b"old").unwrap();
    let result = stores.publish_with(
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        support,
        |point| match crash(point) {
            true => Err(StoreError::Io("crash".into())),
            false => Ok(()),
        },
    );
    assert_eq!(result, Err(StoreError::Io("crash".into())));
    (temp, root)
}

fn recover(root: &Path) -> Result<bool, vpt_application::RepairError> {
    let stores = FilesystemStores::open(&[root.to_path_buf()]).unwrap();
    let target = root.join("note.md");
    repair_publication(&stores, &pending(&stores, &target), &|_| {
        Some(b"intended".to_vec())
    })
}

fn assert_published_without_kept_files(root: &Path) {
    assert_eq!(fs::read(root.join("note.md")).unwrap(), b"intended");
    let names = entries(root);
    assert!(
        !names.iter().any(|name| name.contains("vpt-kept")),
        "{names:?}"
    );
}

#[test]
fn a_crash_while_the_note_is_aside_is_finished_without_the_person() {
    let (_temp, root) = crash_at(ASIDE, |point| matches!(point, Checkpoint::Displaced));
    assert!(!root.join("note.md").exists());
    assert_eq!(recover(&root), Ok(true));
    assert_published_without_kept_files(&root);
}

#[test]
fn a_crash_after_the_swap_removes_the_replaced_version_silently() {
    let (_temp, root) = crash_at(SWAP, |point| matches!(point, Checkpoint::Displaced));
    assert_eq!(fs::read(kept_file(&root)).unwrap(), b"old");
    assert_eq!(recover(&root), Ok(false));
    assert_published_without_kept_files(&root);
}

#[test]
fn a_crash_before_the_swap_removes_vpts_own_bytes_and_republishes() {
    let (_temp, root, _stores) = store();
    fs::write(root.join("note.md"), b"old").unwrap();
    fs::write(root.join(".note.md.vpt-kept-1-1"), b"intended").unwrap();
    assert_eq!(recover(&root), Ok(true));
    assert_published_without_kept_files(&root);
}
