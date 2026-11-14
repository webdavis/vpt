use super::*;

#[test]
fn an_editor_save_inside_the_swap_window_keeps_the_older_save_beside_the_newer_one() {
    let (_temp, root, stores) = store();
    let target = root.join("note.md");
    fs::write(&target, b"old").unwrap();
    let result = publish_swapping(
        &stores,
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        |point| {
            match point {
                Checkpoint::Checked => editor_save(&target, b"first"),
                Checkpoint::Displaced => editor_save(&target, b"second"),
                _ => {}
            }
            Ok(())
        },
    );
    let kept = kept_file(&root);
    assert_eq!(
        result,
        Err(StoreError::EditKept(KeptEdit {
            note: target.clone(),
            kept: kept.clone(),
            versions: KeptVersions::NewerSaveAtNote,
            cause: None,
        }))
    );
    assert_eq!(fs::read(&target).unwrap(), b"second");
    assert_eq!(fs::read(&kept).unwrap(), b"first");
    assert_eq!(entries(&root).len(), 2);
}

#[test]
fn a_newer_save_just_before_the_swap_back_is_kept_and_the_older_save_becomes_the_note() {
    let (_temp, root, stores) = store();
    let target = root.join("note.md");
    fs::write(&target, b"old").unwrap();
    let result = publish_swapping(
        &stores,
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        |point| {
            match point {
                Checkpoint::Checked => editor_save(&target, b"first"),
                Checkpoint::SwapBack => editor_save(&target, b"second"),
                _ => {}
            }
            Ok(())
        },
    );
    let kept = kept_file(&root);
    assert_eq!(
        result,
        Err(StoreError::EditKept(KeptEdit {
            note: target.clone(),
            kept: kept.clone(),
            versions: KeptVersions::OlderSaveAtNote,
            cause: None,
        }))
    );
    assert_eq!(fs::read(&target).unwrap(), b"first");
    assert_eq!(fs::read(&kept).unwrap(), b"second");
    assert_eq!(entries(&root).len(), 2);
}
