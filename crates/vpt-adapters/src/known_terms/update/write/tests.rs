use super::*;
use std::fs;

#[test]
fn staging_failure_preserves_the_original_and_removes_the_temporary() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("terms");
    fs::write(&path, "Before\n").unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    assert_eq!(
        publish_with(
            &root,
            &path,
            Some(digest(b"Before\n")),
            b"Before\nAfter\n",
            |_| { Err(TermsError::Unavailable) }
        ),
        Err(TermsError::Unavailable)
    );
    assert_eq!(fs::read(&path).unwrap(), b"Before\n");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn operator_edit_during_staging_is_preserved_and_the_temporary_is_removed() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("terms");
    fs::write(&path, "Before\n").unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    assert_eq!(
        publish_with(
            &root,
            &path,
            Some(digest(b"Before\n")),
            b"Before\nAfter\n",
            |point| {
                if matches!(point, Checkpoint::Staged) {
                    fs::write(&path, "Operator edit\n").unwrap();
                }
                Ok(())
            }
        ),
        Err(TermsError::Changed)
    );
    assert_eq!(fs::read(&path).unwrap(), b"Operator edit\n");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn uncertain_publication_can_be_retried_without_duplicating_the_term() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("terms");
    let root = RootDir::open(temp.path()).unwrap();
    assert_eq!(
        publish_with(&root, &path, None, b"After\n", |point| {
            if matches!(point, Checkpoint::Renamed) {
                Err(TermsError::Unavailable)
            } else {
                Ok(())
            }
        }),
        Err(TermsError::Unavailable)
    );
    assert_eq!(fs::read(&path).unwrap(), b"After\n");
    publish(&root, &path, None, b"After\n").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"After\n");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn changed_bytes_after_replacement_are_not_acknowledged_as_published() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("terms");
    let root = RootDir::open(temp.path()).unwrap();
    let result = publish_with(&root, &path, None, b"After\n", |point| {
        if matches!(point, Checkpoint::Renamed) {
            fs::write(&path, "Operator edit\n").unwrap();
        }
        Ok(())
    });
    assert_eq!(result, Err(TermsError::Changed));
    assert_eq!(fs::read(&path).unwrap(), b"Operator edit\n");
}

#[test]
fn directory_redirection_removes_only_the_owned_temporary_through_the_retained_descriptor() {
    let temp = crate::test_temp::tempdir().unwrap();
    let parent = temp.path().join("parent");
    let moved = temp.path().join("moved");
    let outside = temp.path().join("outside");
    fs::create_dir(&parent).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("terms"), "Outside\n").unwrap();
    let path = parent.join("terms");
    fs::write(&path, "Before\n").unwrap();
    let root = RootDir::open(&parent).unwrap();
    let result = publish_with(
        &root,
        &path,
        Some(digest(b"Before\n")),
        b"Before\nAfter\n",
        |point| {
            if matches!(point, Checkpoint::Staged) {
                fs::rename(&parent, &moved).unwrap();
                std::os::unix::fs::symlink(&outside, &parent).unwrap();
            }
            Ok(())
        },
    );
    assert_eq!(result, Err(TermsError::Changed));
    assert_eq!(fs::read(moved.join("terms")).unwrap(), b"Before\n");
    assert_eq!(fs::read(outside.join("terms")).unwrap(), b"Outside\n");
    assert_eq!(fs::read_dir(&moved).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
}

#[test]
fn changed_or_replaced_temporary_is_never_published() {
    for replace in [false, true] {
        let temp = crate::test_temp::tempdir().unwrap();
        let path = temp.path().join("terms");
        fs::write(&path, "Before\n").unwrap();
        let root = RootDir::open(temp.path()).unwrap();
        let result = publish_with(
            &root,
            &path,
            Some(digest(b"Before\n")),
            b"Before\nAfter\n",
            |point| {
                if matches!(point, Checkpoint::Staged) {
                    let staged = fs::read_dir(temp.path())
                        .unwrap()
                        .map(|entry| entry.unwrap().path())
                        .find(|entry| {
                            entry
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .starts_with(".vpt-terms-")
                        })
                        .unwrap();
                    if replace {
                        fs::remove_file(&staged).unwrap();
                    }
                    fs::write(staged, "Foreign content\n").unwrap();
                }
                Ok(())
            },
        );
        assert_eq!(result, Err(TermsError::Changed));
        assert_eq!(fs::read(&path).unwrap(), b"Before\n");
        assert_eq!(
            fs::read_dir(temp.path()).unwrap().count(),
            if replace { 2 } else { 1 }
        );
    }
}
