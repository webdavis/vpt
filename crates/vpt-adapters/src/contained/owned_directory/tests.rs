use super::*;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn creates_a_private_directory_exclusively_and_preserves_existing_occupants() {
    let fixture = crate::test_temp::tempdir().unwrap();
    let parent = RootDir::open(fixture.path()).unwrap();
    let directory = parent.create_owned_directory("work").unwrap();
    assert_eq!(
        fs::metadata(directory.path()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    fs::write(directory.path().join("canary"), b"original").unwrap();
    assert!(parent.create_owned_directory("work").is_err());
    assert_eq!(
        fs::read(directory.path().join("canary")).unwrap(),
        b"original"
    );
    assert!(parent.create_owned_directory("../escape").is_err());
}

#[test]
fn removes_owned_temporary_contents_without_following_links() {
    let fixture = crate::test_temp::tempdir().unwrap();
    let outside = crate::test_temp::tempdir().unwrap();
    fs::write(outside.path().join("canary"), b"original").unwrap();
    let parent = RootDir::open(fixture.path()).unwrap();
    let directory = parent.subdirectory("work").unwrap();
    fs::write(directory.path().join("input.m4a"), b"temporary").unwrap();
    let nested = directory.subdirectory("output").unwrap();
    fs::write(nested.path().join("output.json"), b"temporary").unwrap();
    symlink(outside.path(), nested.path().join("directory-link")).unwrap();
    symlink(
        outside.path().join("canary"),
        directory.path().join("file-link"),
    )
    .unwrap();
    parent.remove_owned_directory(&directory).unwrap();
    assert!(!directory.path().exists());
    assert_eq!(
        fs::read(outside.path().join("canary")).unwrap(),
        b"original"
    );
}

#[test]
fn refuses_cleanup_after_the_owned_directory_is_replaced() {
    let fixture = crate::test_temp::tempdir().unwrap();
    let parent = RootDir::open(fixture.path()).unwrap();
    let directory = parent.subdirectory("work").unwrap();
    fs::rename(directory.path(), fixture.path().join("original-work")).unwrap();
    fs::create_dir(directory.path()).unwrap();
    fs::write(directory.path().join("canary"), b"replacement").unwrap();
    assert!(parent.remove_owned_directory(&directory).is_err());
    assert_eq!(
        fs::read(directory.path().join("canary")).unwrap(),
        b"replacement"
    );
    assert!(fixture.path().join("original-work").is_dir());
}

#[test]
fn temporary_cleanup_stops_at_its_entry_and_depth_budgets() {
    let fixture = crate::test_temp::tempdir().unwrap();
    let parent = RootDir::open(fixture.path()).unwrap();
    let directory = parent.create_owned_directory("work").unwrap();
    fs::write(directory.path().join("one"), b"temporary").unwrap();
    fs::write(directory.path().join("two"), b"temporary").unwrap();
    assert!(clear(&directory, &mut 1, 0).is_err());
    assert!(directory.path().join("one").exists());
    parent.remove_owned_directory(&directory).unwrap();
    let nested = parent.create_owned_directory("nested").unwrap();
    let mut deepest = nested.try_clone().unwrap();
    for _ in 0..10 {
        deepest = deepest.subdirectory("child").unwrap();
    }
    fs::write(deepest.path().join("canary"), b"too deep").unwrap();
    assert!(parent.remove_owned_directory(&nested).is_err());
    assert_eq!(
        fs::read(deepest.path().join("canary")).unwrap(),
        b"too deep"
    );
}
