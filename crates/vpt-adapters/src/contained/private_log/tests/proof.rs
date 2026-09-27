use super::*;
use crate::contained::private_log::append::AppendStep;

#[test]
fn failure_preserves_the_inspected_file_identity() {
    let (_temp, root) = fixture();
    plain(&root.path().join("events.jsonl"), b"{}\nfragment", 0o600);
    let mut log = root.open_event_log().expect("private log");
    let metadata = log.file.metadata().expect("held identity");
    let observed = LogInspection {
        device: metadata.dev(),
        inode: metadata.ino(),
        complete_end_offset: 3,
        physical_end_offset: 11,
    };
    let failure = log
        .append_with("{}", |step| {
            if step == AppendStep::Write {
                Err(LogError::Io(ErrorKind::Other))
            } else {
                Ok(())
            }
        })
        .expect_err("write failure");
    assert_eq!(failure.effects.inspection, Some(observed));
    assert_eq!(
        failure.effects.repair,
        Some(TailRepair {
            original_end: 11,
            retained_end: 3,
            synced: true
        })
    );
}

#[test]
fn synchronization_performs_the_real_operation_and_propagates_refusal() {
    use std::os::fd::FromRawFd;
    let mut descriptors = [0; 2];
    // SAFETY: descriptors provides two writable slots; successful pipe transfers both descriptors.
    assert_eq!(unsafe { libc::pipe(descriptors.as_mut_ptr()) }, 0);
    // SAFETY: each successful pipe descriptor transfers once into its owning File.
    let (read, write) = unsafe {
        (
            File::from_raw_fd(descriptors[0]),
            File::from_raw_fd(descriptors[1]),
        )
    };
    let result = crate::contained::private_log::append::synchronize(
        &write,
        AppendStep::FileSync,
        &mut |_| Ok(()),
    );
    drop((read, write));
    assert!(
        result.is_err(),
        "an owned pipe cannot acknowledge file synchronization"
    );
}

#[test]
fn inspection_rejects_leaf_change_before_success() {
    let (_temp, root) = fixture();
    plain(&root.path().join("events.jsonl"), b"{}\n", 0o600);
    let log = root.open_event_log().expect("private log");
    let mut changed = None;
    let result = log.inspect_with(|| {
        fs::rename(root.path().join("events.jsonl"), root.path().join("held"))
            .expect("retain original");
        plain(&root.path().join("events.jsonl"), b"replacement", 0o600);
        changed = Some(snapshot(root.path()));
        Ok(())
    });
    assert_eq!(result, Err(LogError::Unsafe(LogRefusal::Replaced)));
    assert_eq!(
        snapshot(root.path()),
        changed.expect("controlled replacement")
    );
}

#[test]
fn inspection_rejects_state_change_before_success() {
    let (_temp, root) = fixture();
    fs::create_dir(root.path().join("state")).expect("state");
    fs::set_permissions(root.path().join("state"), fs::Permissions::from_mode(0o700))
        .expect("private");
    let state = root.open_subdirectory("state").expect("retained state");
    plain(&state.path().join("events.jsonl"), b"{}\n", 0o600);
    let log = state.open_event_log().expect("private log");
    let mut changed = None;
    let result = log.inspect_with(|| {
        fs::rename(state.path(), root.path().join("held")).expect("retain original");
        fs::create_dir(state.path()).expect("replacement");
        fs::set_permissions(state.path(), fs::Permissions::from_mode(0o700))
            .expect("private replacement");
        changed = Some(snapshot(root.path()));
        Ok(())
    });
    assert!(matches!(
        result,
        Err(LogError::State(PrivateStateError::Contained(
            crate::ContainedError::Escape { .. }
        )))
    ));
    assert_eq!(
        snapshot(root.path()),
        changed.expect("controlled replacement")
    );
}
