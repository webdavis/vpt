use super::{Fixture, SymlinkError, mode, snapshot};
use crate::symlink::{deploy, verify};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[test]
fn creates_only_the_missing_private_leaf_and_link() {
    let fixture = Fixture::new();
    let parent = fixture.target.parent().expect("parent");
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755)).expect("parent mode");
    assert_eq!(deploy(&fixture.link, &fixture.target), Ok(()));
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("link"),
        fixture.target
    );
    assert_eq!(mode(&fixture.target), 0o700);
    assert_eq!(mode(parent), 0o755);
    assert_eq!(verify(&fixture.link, &fixture.target), Ok(()));
}

#[test]
fn existing_target_permissions_are_preserved() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.target).expect("target");
    std::fs::set_permissions(&fixture.target, std::fs::Permissions::from_mode(0o755))
        .expect("mode");
    assert_eq!(deploy(&fixture.link, &fixture.target), Ok(()));
    assert_eq!(mode(&fixture.target), 0o755);
}

#[test]
fn own_absolute_link_is_idempotent_without_replacement() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.target).expect("target");
    fixture.link_to(&fixture.target);
    let before = std::fs::symlink_metadata(&fixture.link).expect("metadata");
    assert_eq!(deploy(&fixture.link, &fixture.target), Ok(()));
    let after = std::fs::symlink_metadata(&fixture.link).expect("metadata");
    assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
}

#[test]
fn equivalent_relative_link_keeps_its_bytes_and_identity() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.target).expect("target");
    fixture.link_to("notes/../notes/vpt");
    let before = std::fs::symlink_metadata(&fixture.link).expect("metadata");
    assert_eq!(deploy(&fixture.link, &fixture.target), Ok(()));
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("preserved link"),
        std::path::PathBuf::from("notes/../notes/vpt")
    );
    let after = std::fs::symlink_metadata(&fixture.link).expect("metadata");
    assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
}

#[test]
fn matching_dangling_link_creates_its_leaf_without_replacing_the_link() {
    let fixture = Fixture::new();
    fixture.link_to("notes/vpt");
    let before = std::fs::symlink_metadata(&fixture.link).expect("metadata");
    assert_eq!(deploy(&fixture.link, &fixture.target), Ok(()));
    assert!(fixture.target.is_dir());
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("link"),
        std::path::PathBuf::from("notes/vpt")
    );
    let after = std::fs::symlink_metadata(&fixture.link).expect("metadata");
    assert_eq!((after.dev(), after.ino()), (before.dev(), before.ino()));
}

#[test]
fn occupied_directory_is_refused_before_target_creation() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.link).expect("occupied");
    assert_eq!(
        deploy(&fixture.link, &fixture.target),
        Err(SymlinkError::Occupied("a directory".into()))
    );
    assert!(!fixture.target.exists());
}

#[test]
fn occupied_file_keeps_its_contents_and_target_absent() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.link, b"private-canary").expect("occupied");
    assert_eq!(
        deploy(&fixture.link, &fixture.target),
        Err(SymlinkError::Occupied("a file".into()))
    );
    assert_eq!(
        std::fs::read(&fixture.link).expect("preserved"),
        b"private-canary"
    );
    assert!(!fixture.target.exists());
}

#[test]
fn different_dangling_link_is_refused_without_creating_a_target() {
    let fixture = Fixture::new();
    fixture.link_to("notes/elsewhere");
    assert_eq!(
        deploy(&fixture.link, &fixture.target),
        Err(SymlinkError::Occupied("a link to notes/elsewhere".into()))
    );
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("link"),
        std::path::PathBuf::from("notes/elsewhere")
    );
    assert!(!fixture.target.exists());
}

#[test]
fn two_unresolved_destinations_are_not_equal() {
    let fixture = Fixture::new();
    fixture.link_to("absent-one/vpt");
    let target = fixture.link.parent().expect("root").join("absent-two/vpt");
    assert!(matches!(
        deploy(&fixture.link, &target),
        Err(SymlinkError::Occupied(_))
    ));
    assert!(!target.parent().expect("parent").exists());
}

#[test]
fn absent_parent_is_refused_without_recursive_creation() {
    let fixture = Fixture::new();
    let target = fixture.target.join("missing/leaf");
    assert_eq!(
        deploy(&fixture.link, &target),
        Err(SymlinkError::TargetParentMissing)
    );
    assert!(!fixture.target.exists());
    assert!(!fixture.link.exists());
}

#[test]
fn file_target_is_refused_and_keeps_its_contents() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.target, b"target-canary").expect("target");
    assert_eq!(
        deploy(&fixture.link, &fixture.target),
        Err(SymlinkError::TargetNotDirectory)
    );
    assert_eq!(
        std::fs::read(&fixture.target).expect("preserved"),
        b"target-canary"
    );
    assert!(!fixture.link.exists());
}

#[test]
fn configured_root_alias_is_resolved_before_link_creation() {
    let fixture = Fixture::new();
    let parent = fixture.target.parent().expect("parent");
    let alias = fixture.link.parent().expect("root").join("vault-alias");
    std::os::unix::fs::symlink(parent, &alias).expect("configured root alias");
    assert_eq!(deploy(&fixture.link, &alias.join("vpt")), Ok(()));
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("link"),
        fixture.target
    );
}

#[test]
fn unreadable_link_is_an_error_before_target_creation() {
    let fixture = Fixture::new();
    let invalid = fixture.link.join("private-canary\0");
    assert_eq!(
        deploy(&invalid, &fixture.target),
        Err(SymlinkError::Io(
            std::io::ErrorKind::InvalidInput.to_string()
        ))
    );
    assert!(!fixture.target.exists());
}

#[test]
fn deploy_reports_target_canonicalization_io_with_existing_link() {
    let fixture = Fixture::new();
    fixture.link_to("notes/vpt");
    let invalid = fixture.target.with_file_name("private-canary\0");
    assert_eq!(
        deploy(&fixture.link, &invalid),
        Err(SymlinkError::Io(
            std::io::ErrorKind::InvalidInput.to_string()
        ))
    );
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("preserved link"),
        std::path::PathBuf::from("notes/vpt")
    );
    assert!(!fixture.target.exists());
}

#[test]
fn deploy_matching_link_refuses_nondirectory_parent_without_writes() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.target, b"parent-canary").expect("regular parent");
    let target = fixture.target.join("child");
    fixture.link_to(&target);
    let before = snapshot(fixture._root.path());
    assert_eq!(
        deploy(&fixture.link, &target),
        Err(SymlinkError::TargetNotDirectory)
    );
    assert_eq!(snapshot(fixture._root.path()), before);
}
