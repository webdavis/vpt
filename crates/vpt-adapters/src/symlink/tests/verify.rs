use super::{Fixture, SymlinkError, mode};
use std::os::unix::fs::PermissionsExt;

#[test]
fn missing_link_is_refused_without_creating_a_target() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.verify(&fixture.link, &fixture.target),
        Err(SymlinkError::Missing)
    );
    assert!(!fixture.target.exists());
}

#[test]
fn matching_absolute_link_passes_without_changing_permissions() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.target).expect("target");
    std::fs::set_permissions(&fixture.target, std::fs::Permissions::from_mode(0o755))
        .expect("mode");
    fixture.link_to(&fixture.target);
    assert_eq!(fixture.verify(&fixture.link, &fixture.target), Ok(()));
    assert_eq!(mode(&fixture.target), 0o755);
}

#[test]
fn relative_equivalent_link_passes_without_rewriting_its_bytes() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.target).expect("target");
    fixture.link_to("notes/../notes/vpt");
    assert_eq!(fixture.verify(&fixture.link, &fixture.target), Ok(()));
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("link"),
        std::path::PathBuf::from("notes/../notes/vpt")
    );
}

#[test]
fn matching_dangling_link_reports_missing_target_without_repair() {
    let fixture = Fixture::new();
    fixture.link_to("notes/vpt");
    assert_eq!(
        fixture.verify(&fixture.link, &fixture.target),
        Err(SymlinkError::TargetMissing)
    );
    assert!(!fixture.target.exists());
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("link"),
        std::path::PathBuf::from("notes/vpt")
    );
}

#[test]
fn different_link_names_the_literal_wrong_target() {
    let fixture = Fixture::new();
    fixture.link_to("notes");
    assert_eq!(
        fixture.verify(&fixture.link, &fixture.target),
        Err(SymlinkError::WrongTarget("notes".into()))
    );
    assert!(!fixture.target.exists());
}

#[test]
fn two_unresolved_targets_are_refused_as_different() {
    let fixture = Fixture::new();
    fixture.link_to("absent-one/vpt");
    let target = fixture.link.parent().expect("root").join("absent-two/vpt");
    assert_eq!(
        fixture.verify(&fixture.link, &target),
        Err(SymlinkError::WrongTarget("absent-one/vpt".into()))
    );
    assert!(!target.exists());
}

#[test]
fn regular_file_target_is_refused_without_changing_bytes() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.target, b"target-canary").expect("target");
    fixture.link_to(&fixture.target);
    assert_eq!(
        fixture.verify(&fixture.link, &fixture.target),
        Err(SymlinkError::TargetNotDirectory)
    );
    assert_eq!(
        std::fs::read(&fixture.target).expect("preserved"),
        b"target-canary"
    );
}

#[test]
fn occupied_directory_is_refused_without_replacement() {
    let fixture = Fixture::new();
    std::fs::create_dir(&fixture.link).expect("occupied");
    assert_eq!(
        fixture.verify(&fixture.link, &fixture.target),
        Err(SymlinkError::Occupied("a directory".into()))
    );
    assert!(fixture.link.is_dir());
    assert!(!fixture.target.exists());
}

#[test]
fn occupied_file_is_refused_without_reading_or_replacing_it() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.link, b"private-canary").expect("occupied");
    assert_eq!(
        fixture.verify(&fixture.link, &fixture.target),
        Err(SymlinkError::Occupied("a file".into()))
    );
    assert_eq!(
        std::fs::read(&fixture.link).expect("preserved"),
        b"private-canary"
    );
}

#[test]
fn unreadable_link_is_an_error_not_a_missing_link() {
    let fixture = Fixture::new();
    let invalid = fixture.link.join("private-canary\0");
    assert_eq!(
        fixture.verify(&invalid, &fixture.target),
        Err(SymlinkError::Io(
            std::io::ErrorKind::InvalidInput.to_string()
        ))
    );
    assert!(!fixture.target.exists());
}

#[test]
fn verify_reports_target_canonicalization_io_with_existing_link() {
    let fixture = Fixture::new();
    fixture.link_to("notes/vpt");
    let invalid = fixture.target.with_file_name("private-canary\0");
    assert_eq!(
        fixture.verify(&fixture.link, &invalid),
        Err(SymlinkError::Io(
            std::io::ErrorKind::InvalidInput.to_string()
        ))
    );
    assert_eq!(
        std::fs::read_link(&fixture.link).expect("preserved link"),
        std::path::PathBuf::from("notes/vpt")
    );
}

#[test]
fn verify_matching_link_refuses_nondirectory_parent_without_writes() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.target, b"parent-canary").expect("regular parent");
    let target = fixture.target.join("child");
    fixture.link_to(&target);
    assert_eq!(
        fixture.verify(&fixture.link, &target),
        Err(SymlinkError::TargetNotDirectory)
    );
}
