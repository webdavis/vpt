use super::*;

#[test]
fn writer_waits_for_the_previous_lock_to_be_released() {
    let temp = tempfile::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let held = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        drop(held);
    });
    let result = lock(&root);
    release.join().unwrap();
    assert!(
        result.is_ok(),
        "the next writer should wait for the held lock"
    );
}

#[test]
fn writer_contention_is_an_operational_error() {
    let temp = tempfile::tempdir().unwrap();
    let root = RootDir::open(temp.path()).unwrap();
    let _held = WriteLock::acquire(&root, Duration::ZERO).unwrap();
    let error = lock_with_wait(&root, Duration::ZERO)
        .err()
        .expect("held lock");
    assert_eq!(error.exit_code(), 1);
    assert!(error.rule.is_none());
}
