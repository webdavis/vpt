use super::*;

#[test]
fn absent_log_is_exclusively_created_private() {
    let (_temp, root) = fixture();
    let log = root.open_event_log();
    assert!(log.is_ok(), "private creation must succeed: {log:?}");
    let metadata = fs::metadata(root.path().join("events.jsonl")).expect("created log");
    assert!(metadata.is_file());
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.uid(), effective_user());
    assert_eq!(metadata.nlink(), 1);
    assert_eq!(metadata.len(), 0);
}

#[test]
fn existing_private_log_is_opened_without_mutation() {
    let (_temp, root) = fixture();
    plain(&root.path().join("events.jsonl"), b"existing\n", 0o600);
    let before = snapshot(root.path());
    let result = root.open_event_log();
    assert_eq!(snapshot(root.path()), before);
    assert!(result.is_ok(), "private existing log must open: {result:?}");
}

#[test]
fn shared_state_refuses_log_creation_without_changes() {
    let (_temp, root) = fixture();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o750)).expect("shared");
    let before = snapshot(root.path());
    let result = root.open_event_log().err();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(result, Some(LogError::State(PrivateStateError::Mode)));
}

#[test]
fn linked_log_refuses_without_touching_the_referent() {
    let (_temp, root) = fixture();
    let target = root.path().join("referent");
    plain(&target, b"referent canary", 0o600);
    symlink(&target, root.path().join("events.jsonl")).expect("owned link");
    let before = snapshot(root.path());
    let result = root.open_event_log().err();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(result, Some(LogError::Unsafe(LogRefusal::NotRegular)));
}

#[test]
fn directory_log_refuses_without_changes() {
    let (_temp, root) = fixture();
    fs::create_dir(root.path().join("events.jsonl")).expect("owned directory");
    let before = snapshot(root.path());
    let result = root.open_event_log().err();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(result, Some(LogError::Unsafe(LogRefusal::NotRegular)));
}

#[test]
fn hard_linked_log_refuses_without_changes() {
    let (_temp, root) = fixture();
    let target = root.path().join("original");
    plain(&target, b"shared inode canary", 0o600);
    fs::hard_link(&target, root.path().join("events.jsonl")).expect("owned hard link");
    let before = snapshot(root.path());
    let result = root.open_event_log().err();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(result, Some(LogError::Unsafe(LogRefusal::MultipleLinks)));
}

#[test]
fn log_modes_are_refused_without_normalization() {
    let (_temp, root) = fixture();
    let path = root.path().join("events.jsonl");
    plain(&path, b"mode canary", 0o600);
    for mode in [0o640, 0o400, 0o644, 0o1600] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("owned mode");
        let before = snapshot(root.path());
        let result = root.open_event_log().err();
        assert_eq!(snapshot(root.path()), before);
        assert_eq!(
            result,
            Some(LogError::Unsafe(LogRefusal::Mode)),
            "mode {mode:o}"
        );
    }
}

#[test]
fn file_owner_check_uses_the_effective_identity() {
    let (_temp, root) = fixture();
    let path = root.path().join("events.jsonl");
    plain(&path, b"owner canary", 0o600);
    let file = File::open(&path).expect("owned file");
    let before = snapshot(root.path());
    let result = verify_file(&root, &file, effective_user().wrapping_add(1));
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(result, Err(LogError::Unsafe(LogRefusal::Owner)));
}

#[test]
fn retained_file_refuses_a_replacement_name() {
    let (_temp, root) = fixture();
    let path = root.path().join("events.jsonl");
    plain(&path, b"held canary", 0o600);
    let file = File::open(&path).expect("retained file");
    fs::rename(&path, root.path().join("held")).expect("retain original");
    plain(&path, b"replacement canary", 0o600);
    let before = snapshot(root.path());
    let result = verify_file(&root, &file, effective_user());
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(result, Err(LogError::Unsafe(LogRefusal::Replaced)));
}

#[test]
fn restrictive_creation_mode_is_normalized_only_for_the_new_owned_file() {
    let (_temp, root) = fixture();
    let result = root.open_event_log_with_mode(0o400);
    assert!(
        result.is_ok(),
        "exclusive new descriptor must become 0600: {result:?}"
    );
    let metadata = fs::metadata(root.path().join("events.jsonl")).expect("new log");
    assert_eq!(metadata.mode() & 0o7777, 0o600);
    assert_eq!(metadata.uid(), effective_user());
    assert_eq!(metadata.nlink(), 1);
    let before = snapshot(root.path());
    assert!(root.open_event_log_with_mode(0o400).is_ok());
    assert_eq!(snapshot(root.path()), before);
}
