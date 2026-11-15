use super::*;

#[test]
fn writer_waits_for_the_previous_lock_to_be_released() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let held = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        drop(held);
    });
    let result = lock_with_wait(&root, Duration::from_secs(5));
    release.join().unwrap();
    assert!(
        result.is_ok(),
        "the next writer should wait for the held lock"
    );
}

#[test]
fn writer_contention_is_an_operational_error() {
    let temp = crate::test_temp::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let _held = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let error = lock_with_wait(&root, Duration::ZERO)
        .err()
        .expect("held lock");
    assert_eq!(error.exit_code(), 1);
    assert!(error.rule.is_none());
}

#[test]
fn a_state_folder_with_the_wrong_mode_is_refused_with_the_command_that_fixes_it() {
    let error = private_state_error(Path::new("/data/team's vpt"), PrivateStateError::Mode);
    assert_eq!(error.rule.as_deref(), Some("unsafe_state"));
    assert_eq!(
        error.message,
        r"state directory '/data/team'\''s vpt' must have mode 0700; run: chmod 700 '/data/team'\''s vpt'"
    );
}

#[test]
fn a_state_folder_owned_by_another_user_is_refused_without_a_chmod() {
    let error = private_state_error(Path::new("/data/vpt"), PrivateStateError::Owner);
    assert_eq!(error.rule.as_deref(), Some("unsafe_state"));
    assert_eq!(
        error.message,
        "state directory '/data/vpt' is owned by another user; it must be owned by the current user with mode 0700"
    );
}
