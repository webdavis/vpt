use super::*;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

fn root() -> (tempfile::TempDir, RootDir) {
    let temp = crate::test_temp::tempdir().expect("owned state");
    let path = temp.path().canonicalize().expect("canonical state");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private");
    let root = RootDir::open(&path).expect("retained state");
    (temp, root)
}

fn metadata(root: &RootDir) -> (u64, u64, u32, u32, i64, i64) {
    let metadata = fs::metadata(root.path()).expect("metadata");
    (
        metadata.dev(),
        metadata.ino(),
        metadata.uid(),
        metadata.mode(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[test]
fn private_state_validation_is_read_only() {
    let (_temp, root) = root();
    let before = metadata(&root);
    assert_eq!(root.validate_private_state(), Ok(()));
    assert_eq!(metadata(&root), before);
    assert_eq!(fs::read_dir(root.path()).expect("contents").count(), 0);
}

#[test]
fn existing_state_modes_are_refused_without_normalization() {
    let (_temp, root) = root();
    for mode in [0o600, 0o750, 0o755, 0o1700] {
        fs::set_permissions(root.path(), fs::Permissions::from_mode(mode)).expect("mode");
        let before = metadata(&root);
        let result = root.validate_private_state();
        assert_eq!(metadata(&root), before);
        assert_eq!(result, Err(PrivateStateError::Mode), "mode {mode:o}");
    }
}

#[test]
fn state_owner_must_match_the_effective_identity() {
    let (_temp, root) = root();
    let before = metadata(&root);
    let result = validate_as(&root, effective_user().wrapping_add(1));
    assert_eq!(metadata(&root), before);
    assert_eq!(result, Err(PrivateStateError::Owner));
}

#[test]
fn retained_state_rejects_a_replacement_directory() {
    let temp = crate::test_temp::tempdir().expect("owned fixture");
    let base = temp.path().canonicalize().expect("canonical base");
    let path = base.join("state");
    fs::create_dir(&path).expect("state");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private");
    let root = RootDir::open(&path).expect("retained state");
    fs::rename(&path, base.join("held")).expect("retain original");
    fs::create_dir(&path).expect("replacement");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private");
    let before = metadata(&root);
    let result = root.validate_private_state();
    assert_eq!(metadata(&root), before);
    assert!(matches!(
        result,
        Err(PrivateStateError::Contained(ContainedError::Escape { .. }))
    ));
    assert_eq!(
        fs::read_dir(&path).expect("replacement contents").count(),
        0
    );
    assert_eq!(
        fs::read_dir(base.join("held"))
            .expect("held contents")
            .count(),
        0
    );
}
