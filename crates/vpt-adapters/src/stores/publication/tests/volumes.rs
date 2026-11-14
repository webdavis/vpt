use super::*;
use std::io::{Seek, Write};

pub(super) const NO_SWAP: [RenameSupport; 2] = [
    RenameSupport {
        swap: false,
        exclusive: true,
        hard_links: true,
    },
    RenameSupport {
        swap: false,
        exclusive: false,
        hard_links: true,
    },
];

fn replace_old(
    support: RenameSupport,
    edit: impl FnMut(Checkpoint, &Path),
) -> (tempfile::TempDir, PathBuf, Result<(), StoreError>) {
    let (temp, root, stores) = store();
    let target = root.join("note.md");
    fs::write(&target, b"old").unwrap();
    let mut edit = edit;
    let result = stores.publish_with(
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        support,
        |point| {
            edit(point, &target);
            Ok(())
        },
    );
    (temp, root, result)
}

#[test]
fn a_test_directory_reports_swap_and_a_device_directory_does_not() {
    let (_temp, root, _stores) = store();
    assert_eq!(RootDir::open(&root).unwrap().rename_support(), SWAP);
    assert!(
        !RootDir::open(Path::new("/dev"))
            .unwrap()
            .rename_support()
            .swap
    );
}

#[test]
fn every_volume_kind_replaces_an_unchanged_note_without_leftovers() {
    for support in [SWAP].into_iter().chain(NO_SWAP) {
        let (_temp, root, result) = replace_old(support, |_, _| {});
        assert_eq!(result, Ok(()), "{support:?}");
        assert_eq!(fs::read(root.join("note.md")).unwrap(), b"intended");
        assert_eq!(entries(&root), ["note.md"], "{support:?}");
    }
}

#[test]
fn without_swap_an_edit_is_hashed_aside_and_put_back_never_overwritten() {
    for support in NO_SWAP {
        let (_temp, root, result) = replace_old(support, |point, target| {
            if matches!(point, Checkpoint::Checked) {
                editor_save(target, b"operator");
            }
        });
        assert_eq!(result, Err(StoreError::Changed(root.join("note.md"))));
        assert_eq!(fs::read(root.join("note.md")).unwrap(), b"operator");
        assert_eq!(entries(&root), ["note.md"], "{support:?}");
    }
}

#[test]
fn without_swap_a_save_while_the_note_is_aside_is_never_replaced() {
    for support in NO_SWAP {
        let (_temp, root, result) = replace_old(support, |point, target| match point {
            Checkpoint::Checked => editor_save(target, b"first"),
            Checkpoint::Displaced => editor_save(target, b"second"),
            _ => {}
        });
        let kept = kept_file(&root);
        assert_eq!(
            result,
            Err(StoreError::EditKept(KeptEdit {
                note: root.join("note.md"),
                kept: kept.clone(),
                versions: KeptVersions::NewerSaveAtNote,
                cause: None,
            })),
            "{support:?}"
        );
        assert_eq!(fs::read(root.join("note.md")).unwrap(), b"second");
        assert_eq!(fs::read(kept).unwrap(), b"first");
        assert_eq!(entries(&root).len(), 2, "{support:?}");
    }
}

#[test]
fn a_write_through_a_held_descriptor_after_hashing_is_not_unlinked() {
    for support in [SWAP].into_iter().chain(NO_SWAP) {
        let mut held = None;
        let (_temp, root, result) = replace_old(support, |point, target| match point {
            Checkpoint::Staged => {
                held = Some(fs::OpenOptions::new().write(true).open(target).unwrap())
            }
            Checkpoint::Hashed => {
                let file = held.as_mut().unwrap();
                file.rewind().unwrap();
                file.write_all(b"OPERATOR").unwrap();
            }
            _ => {}
        });
        if support.swap {
            assert_eq!(result, Err(StoreError::Changed(root.join("note.md"))));
            assert_eq!(fs::read(root.join("note.md")).unwrap(), b"OPERATOR");
            assert_eq!(entries(&root), ["note.md"]);
        } else {
            let kept = kept_file(&root);
            assert_eq!(
                result,
                Err(StoreError::EditKept(KeptEdit {
                    note: root.join("note.md"),
                    kept: kept.clone(),
                    versions: KeptVersions::PublishedAtNote,
                    cause: None,
                })),
                "{support:?}"
            );
            assert_eq!(fs::read(root.join("note.md")).unwrap(), b"intended");
            assert_eq!(fs::read(kept).unwrap(), b"OPERATOR");
        }
    }
}
