use super::*;
mod bounds;
mod recovery;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

#[test]
fn appends_one_normalized_line_and_preserves_existing_bytes() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("known-terms.txt");
    let original = "  Straße  \r\nNew\tYork\nAnn,";
    fs::write(&path, original).unwrap();
    let update = prepare(&path, "  Private\r\n  Name\t Jr.  ").unwrap();
    assert!(update.added());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(!format!("{update:?}").contains("Private"));
    update.publish().unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        format!("{original}\nPrivate Name Jr.\n")
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    update.publish().unwrap();
    assert_eq!(
        super::super::read(&path).unwrap(),
        ["Straße", "New\tYork", "Ann,", "Private Name Jr."]
    );
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn duplicate_whole_terms_preserve_spelling_bytes_and_mtime() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("terms");
    fs::write(&path, " Straße \r\nNew\tYork\nAnn,\n").unwrap();
    let original = fs::read(&path).unwrap();
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    for text in ["STRASSE", "new  york", "Ａｎｎ,"] {
        let update = prepare(&path, text).unwrap();
        assert!(!update.added());
        update.publish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
    }
    let distinct = prepare(&path, "Ann").unwrap();
    assert!(distinct.added());
    distinct.publish().unwrap();
    assert!(fs::read_to_string(&path).unwrap().ends_with("Ann,\nAnn\n"));
}

#[test]
fn planning_missing_parents_is_observational_and_publication_creates_private_directories() {
    let temp = crate::test_temp::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
    let path = temp.path().join("new/config/terms");
    let update = prepare(&path, "Alice").unwrap();
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    update.publish().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "Alice\n");
    assert_eq!(
        fs::metadata(temp.path()).unwrap().permissions().mode() & 0o777,
        0o755
    );
    for parent in [
        path.parent().unwrap(),
        path.parent().unwrap().parent().unwrap(),
    ] {
        assert_eq!(
            fs::metadata(parent).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}

#[test]
fn changed_deleted_and_newly_created_targets_are_preserved() {
    for scenario in 0..3 {
        let temp = crate::test_temp::tempdir().unwrap();
        let path = temp.path().join("terms");
        if scenario != 2 {
            fs::write(&path, "Before\n").unwrap();
        }
        let update = prepare(&path, "After").unwrap();
        if scenario == 1 {
            fs::remove_file(&path).unwrap();
        } else {
            fs::write(&path, "Operator edit\n").unwrap();
        }
        assert_eq!(update.publish(), Err(TermsError::Changed));
        if scenario == 1 {
            assert!(!path.exists());
        } else {
            assert_eq!(fs::read_to_string(&path).unwrap(), "Operator edit\n");
        }
        assert_eq!(
            fs::read_dir(temp.path()).unwrap().count(),
            usize::from(scenario != 1)
        );
    }
}

#[test]
fn paths_redirected_after_planning_are_refused_without_following_them() {
    let temp = crate::test_temp::tempdir().unwrap();
    fs::create_dir(temp.path().join("parent")).unwrap();
    fs::create_dir(temp.path().join("outside")).unwrap();
    let path = temp.path().join("parent/terms");
    let update = prepare(&path, "Private").unwrap();
    fs::rename(temp.path().join("parent"), temp.path().join("moved")).unwrap();
    symlink(temp.path().join("outside"), temp.path().join("parent")).unwrap();
    assert!(update.publish().is_err());
    assert_eq!(
        fs::read_dir(temp.path().join("outside")).unwrap().count(),
        0
    );
    assert_eq!(fs::read_dir(temp.path().join("moved")).unwrap().count(), 0);
}

#[test]
fn invalid_terms_and_unsafe_existing_lists_leave_no_artifacts() {
    let temp = crate::test_temp::tempdir().unwrap();
    let path = temp.path().join("terms");
    for term in [
        "".into(),
        " \t\n".into(),
        "private\0".into(),
        "x".repeat(65_537),
    ] {
        assert_eq!(prepare(&path, &term).unwrap_err(), TermsError::InvalidText);
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    fs::write(&path, "canary\0private").unwrap();
    assert_eq!(prepare(&path, "Term").unwrap_err(), TermsError::InvalidText);
    fs::write(&path, "Original\n").unwrap();
    let link = temp.path().join("link");
    fs::hard_link(&path, &link).unwrap();
    assert_eq!(prepare(&path, "Term").unwrap_err(), TermsError::UnsafePath);
    assert_eq!(fs::read_to_string(&path).unwrap(), "Original\n");
    fs::remove_file(link).unwrap();
    symlink(&path, temp.path().join("symlink")).unwrap();
    assert!(prepare(&temp.path().join("symlink"), "Term").is_err());
    assert!(prepare(Path::new("relative"), "Term").is_err());
}
