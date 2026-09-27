use super::*;

fn log(root: &RootDir, bytes: &[u8]) -> EventLog {
    plain(&root.path().join("events.jsonl"), bytes, 0o600);
    root.open_event_log().expect("private log")
}

#[test]
fn inspection_distinguishes_complete_prefix_from_partial_tail_without_writes() {
    let (_temp, root) = fixture();
    let log = log(&root, b"{}\n{}\nfragment");
    let identity = log.file.metadata().expect("identity");
    let before = snapshot(root.path());
    let result = log.inspect();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(
        result,
        Ok(LogInspection {
            device: identity.dev(),
            inode: identity.ino(),
            complete_end_offset: 6,
            physical_end_offset: 14
        })
    );
}

#[test]
fn inspection_of_no_newline_tail_does_not_repair_it() {
    let (_temp, root) = fixture();
    let log = log(&root, b"fragment");
    let before = snapshot(root.path());
    let result = log.inspect();
    assert_eq!(snapshot(root.path()), before);
    let result = result.expect("inspect tail");
    assert_eq!(
        (result.complete_end_offset, result.physical_end_offset),
        (0, 8)
    );
}

#[test]
fn inspection_of_empty_and_complete_files_has_no_partial_tail() {
    let (_temp, root) = fixture();
    for bytes in [b"".as_slice(), b"{}\n".as_slice()] {
        let log = log(&root, bytes);
        let before = snapshot(root.path());
        let result = log.inspect();
        assert_eq!(snapshot(root.path()), before);
        let result = result.expect("inspect complete log");
        assert_eq!(result.complete_end_offset, bytes.len() as u64);
        assert_eq!(result.physical_end_offset, bytes.len() as u64);
    }
}

#[test]
fn inspection_refuses_replaced_leaf_without_changes() {
    let (_temp, root) = fixture();
    let log = log(&root, b"held\n");
    fs::rename(root.path().join("events.jsonl"), root.path().join("held"))
        .expect("retain original");
    plain(
        &root.path().join("events.jsonl"),
        b"replacement canary",
        0o600,
    );
    let before = snapshot(root.path());
    let result = log.inspect();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(result, Err(LogError::Unsafe(LogRefusal::Replaced)));
}

#[test]
fn inspection_refuses_replaced_state_without_changes() {
    let (_temp, root) = fixture();
    fs::create_dir(root.path().join("state")).expect("state");
    fs::set_permissions(root.path().join("state"), fs::Permissions::from_mode(0o700))
        .expect("private state");
    let state = root.open_subdirectory("state").expect("retained state");
    let log = log(&state, b"held\n");
    fs::rename(state.path(), root.path().join("held")).expect("retain original");
    fs::create_dir(state.path()).expect("replacement");
    fs::set_permissions(state.path(), fs::Permissions::from_mode(0o700))
        .expect("private replacement");
    let before = snapshot(root.path());
    let result = log.inspect();
    assert_eq!(snapshot(root.path()), before);
    assert!(matches!(
        result,
        Err(LogError::State(PrivateStateError::Contained(
            crate::ContainedError::Escape { .. }
        )))
    ));
}
