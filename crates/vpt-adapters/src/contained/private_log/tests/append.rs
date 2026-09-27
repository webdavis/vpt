use super::*;
use crate::contained::private_log::append::AppendStep;

fn log(root: &RootDir, bytes: &[u8]) -> EventLog {
    plain(&root.path().join("events.jsonl"), bytes, 0o600);
    root.open_event_log().expect("acquire private log")
}

#[test]
fn append_receipt_requires_file_then_directory_sync() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"old\n");
    let identity = log.file.metadata().expect("retained identity");
    let mut stages = Vec::new();
    let result = log.append_with("new", |step| {
        stages.push(step);
        Ok(())
    });
    let receipt = result.expect("complete synced append");
    assert_eq!(
        receipt,
        AppendReceipt {
            device: identity.dev(),
            inode: identity.ino(),
            end_offset: 8,
            repair: None
        }
    );
    assert_eq!(
        stages,
        [
            AppendStep::Write,
            AppendStep::Write,
            AppendStep::FileSync,
            AppendStep::DirectorySync,
            AppendStep::Confirm
        ]
    );
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"old\nnew\n"
    );
}

#[test]
fn empty_file_append_has_no_tail_repair() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"");
    let receipt = log.append_line("{}").expect("append empty file");
    assert_eq!(receipt.end_offset, 3);
    assert_eq!(receipt.repair, None);
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n"
    );
}

#[test]
fn tail_only_file_is_repaired_before_append() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"{\"unfinished\":");
    let receipt = log.append_line("{}").expect("repair and append");
    assert_eq!(
        receipt.repair,
        Some(TailRepair {
            original_end: 14,
            retained_end: 0,
            synced: true
        })
    );
    assert_eq!(receipt.end_offset, 3);
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n"
    );
}

#[test]
fn complete_lines_survive_fragment_repair() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"{}\n{}\nfragment");
    let receipt = log.append_line("{\"next\":1}").expect("repair and append");
    assert_eq!(
        receipt.repair,
        Some(TailRepair {
            original_end: 14,
            retained_end: 6,
            synced: true
        })
    );
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n{}\n{\"next\":1}\n"
    );
}

#[test]
fn incomplete_tail_spanning_read_chunks_preserves_complete_prefix() {
    let (_temp, root) = fixture();
    let mut bytes = b"{}\n".to_vec();
    bytes.extend(std::iter::repeat_n(b'x', 20_000));
    let mut log = log(&root, &bytes);
    let receipt = log.append_line("{}").expect("bounded chunk repair");
    assert_eq!(
        receipt.repair,
        Some(TailRepair {
            original_end: 20_003,
            retained_end: 3,
            synced: true
        })
    );
    assert_eq!(
        fs::read(root.path().join("events.jsonl")).expect("bytes"),
        b"{}\n{}\n"
    );
}

#[test]
fn invalid_lines_change_nothing_including_an_incomplete_tail() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"fragment");
    for line in ["", "one\ntwo", "one\rtwo", "{}\n"] {
        let before = snapshot(root.path());
        let result = log.append_line(line);
        assert_eq!(snapshot(root.path()), before);
        assert_eq!(
            result.map_err(|failure| *failure),
            Err(AppendFailure {
                error: LogError::InvalidLine,
                effects: AppendEffects::default()
            })
        );
    }
}

#[test]
fn reopened_unsynced_creation_still_syncs_the_directory() {
    let (_temp, root) = fixture();
    drop(root.open_event_log().expect("exclusive creation"));
    let mut reopened = root.open_event_log().expect("reopen created log");
    let mut stages = Vec::new();
    let result = reopened.append_with("{}", |step| {
        stages.push(step);
        Ok(())
    });
    assert!(result.is_ok(), "reopened append: {result:?}");
    assert_eq!(
        stages,
        [
            AppendStep::Write,
            AppendStep::Write,
            AppendStep::FileSync,
            AppendStep::DirectorySync,
            AppendStep::Confirm
        ]
    );
}

#[test]
fn replaced_leaf_is_refused_before_append() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"held\n");
    fs::rename(root.path().join("events.jsonl"), root.path().join("held"))
        .expect("retain original");
    plain(&root.path().join("events.jsonl"), b"replacement\n", 0o600);
    let before = snapshot(root.path());
    let result = log.append_line("private canary");
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(
        result.map_err(|failure| *failure),
        Err(AppendFailure {
            error: LogError::Unsafe(LogRefusal::Replaced),
            effects: AppendEffects::default()
        })
    );
}

#[test]
fn shared_leaf_is_refused_before_append() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"held\n");
    fs::set_permissions(
        root.path().join("events.jsonl"),
        fs::Permissions::from_mode(0o640),
    )
    .expect("changed mode");
    let before = snapshot(root.path());
    let result = log.append_line("private canary");
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(
        result.map_err(|failure| *failure),
        Err(AppendFailure {
            error: LogError::Unsafe(LogRefusal::Mode),
            effects: AppendEffects::default()
        })
    );
}

#[test]
fn shared_state_is_refused_before_append() {
    let (_temp, root) = fixture();
    let mut log = log(&root, b"held\n");
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o750)).expect("changed mode");
    let before = snapshot(root.path());
    let result = log.append_line("private canary");
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(
        result.map_err(|failure| *failure),
        Err(AppendFailure {
            error: LogError::State(PrivateStateError::Mode),
            effects: AppendEffects::default()
        })
    );
}

#[test]
fn replaced_state_is_refused_before_append() {
    let (_temp, root) = fixture();
    fs::create_dir(root.path().join("state")).expect("state");
    fs::set_permissions(root.path().join("state"), fs::Permissions::from_mode(0o700))
        .expect("private");
    let state = root.open_subdirectory("state").expect("retained state");
    let mut log = log(&state, b"held\n");
    fs::rename(state.path(), root.path().join("held")).expect("retain original");
    fs::create_dir(state.path()).expect("replacement");
    fs::set_permissions(state.path(), fs::Permissions::from_mode(0o700))
        .expect("private replacement");
    let before = snapshot(root.path());
    let result = log.append_line("private canary");
    assert_eq!(snapshot(root.path()), before);
    assert!(matches!(
        result.map_err(|failure| *failure),
        Err(AppendFailure {
            error: LogError::State(PrivateStateError::Contained(
                crate::ContainedError::Escape { .. }
            )),
            effects: AppendEffects {
                written_bytes: 0,
                repair: None,
                ..
            }
        })
    ));
}
