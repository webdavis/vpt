use super::*;
use crate::stores::tests::{entries, store};
use std::{fs, fs::hard_link, os::unix::fs::symlink};

#[test]
fn a_new_target_after_the_final_check_is_never_replaced() {
    for kind in ["file", "directory", "link"] {
        let (temp, root, stores) = store();
        let target = root.join("note.md");
        let outside = temp.path().join("outside");
        fs::write(&outside, b"external").unwrap();
        let result = stores.publish_with(&target, None, b"intended", |point| {
            if matches!(point, Checkpoint::Checked) {
                match kind {
                    "file" => fs::write(&target, b"operator").unwrap(),
                    "directory" => fs::create_dir(&target).unwrap(),
                    _ => symlink(&outside, &target).unwrap(),
                }
            }
            Ok(())
        });
        assert_eq!(result, Err(StoreError::Changed(target.clone())), "{kind}");
        match kind {
            "file" => assert_eq!(fs::read(target).unwrap(), b"operator"),
            "directory" => assert!(target.is_dir()),
            _ => assert_eq!(fs::read_link(target).unwrap(), outside),
        }
        assert_eq!(fs::read(outside).unwrap(), b"external");
    }
}

#[test]
fn existing_target_edits_during_staging_are_preserved() {
    let (_temp, root, stores) = store();
    let target = root.join("note.md");
    fs::write(&target, b"old").unwrap();
    let result = stores.publish_with(
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        |point| {
            if matches!(point, Checkpoint::Staged) {
                fs::write(&target, b"operator").unwrap();
            }
            Ok(())
        },
    );
    assert_eq!(result, Err(StoreError::Changed(target.clone())));
    assert_eq!(fs::read(target).unwrap(), b"operator");
}

#[test]
fn changed_or_replaced_staged_bytes_are_never_published() {
    for kind in ["edited", "extended", "replaced", "linked", "symlink"] {
        let (temp, root, stores) = store();
        let target = root.join("note.md");
        fs::write(&target, b"old").unwrap();
        let outside = temp.path().join("outside");
        fs::write(&outside, b"external").unwrap();
        let result = stores.publish_with(
            &target,
            Some(stores.digest_bytes(b"old")),
            b"intended",
            |point| {
                if matches!(point, Checkpoint::Staged) {
                    let name = entries(&root)
                        .into_iter()
                        .find(|name| name.starts_with('.'))
                        .unwrap();
                    let staged = root.join(name);
                    match kind {
                        "edited" => fs::write(&staged, b"tampered").unwrap(),
                        "extended" => fs::write(&staged, b"intended extra bytes").unwrap(),
                        "replaced" => {
                            fs::remove_file(&staged).unwrap();
                            fs::write(&staged, b"intended").unwrap();
                        }
                        "linked" => hard_link(&staged, root.join("alias")).unwrap(),
                        _ => {
                            fs::remove_file(&staged).unwrap();
                            symlink(&outside, &staged).unwrap();
                        }
                    }
                }
                Ok(())
            },
        );
        assert!(
            matches!(result, Err(StoreError::Changed(_))),
            "{kind}: {result:?}"
        );
        assert_eq!(fs::read(target).unwrap(), b"old");
        assert_eq!(fs::read(outside).unwrap(), b"external");
    }
}

#[test]
fn root_replacement_during_staging_preserves_both_original_and_replacement_targets() {
    let (temp, root, stores) = store();
    let target = root.join("note.md");
    let held = temp.path().join("held");
    fs::write(&target, b"old").unwrap();
    let result = stores.publish_with(
        &target,
        Some(stores.digest_bytes(b"old")),
        b"intended",
        |point| {
            if matches!(point, Checkpoint::Staged) {
                fs::rename(&root, &held).unwrap();
                fs::create_dir(&root).unwrap();
                fs::write(&target, b"replacement").unwrap();
            }
            Ok(())
        },
    );
    assert!(matches!(result, Err(StoreError::Escape(_))));
    assert_eq!(fs::read(held.join("note.md")).unwrap(), b"old");
    assert_eq!(fs::read(target).unwrap(), b"replacement");
}

#[test]
fn changes_after_rename_are_preserved_and_reported_as_incomplete() {
    let (_temp, root, stores) = store();
    let target = root.join("note.md");
    let result = stores.publish_with(&target, None, b"intended", |point| {
        if matches!(point, Checkpoint::Renamed) {
            fs::write(&target, b"operator").unwrap();
        }
        Ok(())
    });
    assert_eq!(result, Err(StoreError::Changed(target.clone())));
    assert_eq!(fs::read(target).unwrap(), b"operator");
}

#[test]
fn temporary_name_collisions_have_a_bounded_attempt_count_and_preserve_occupants() {
    let (_temp, root, stores) = store();
    for sequence in 0..64 {
        fs::write(
            root.join(format!(".note.md.vpt-{}-{sequence}", std::process::id())),
            b"occupant",
        )
        .unwrap();
    }
    assert_eq!(
        stores.publish(&root.join("note.md"), None, b"intended"),
        Err(StoreError::Io(
            "publication temporary names exhausted".into()
        ))
    );
    assert_eq!(entries(&root).len(), 64);
    for name in entries(&root) {
        assert_eq!(fs::read(root.join(name)).unwrap(), b"occupant");
    }
}
