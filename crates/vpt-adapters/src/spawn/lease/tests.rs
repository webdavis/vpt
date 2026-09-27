use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::TryLockError;

#[test]
fn authority_is_exclusive_until_the_permit_is_released() {
    let lease = SignalLease::new(42);
    let peer = lease.clone();
    let permit = lease.enter().expect("retained authority entered");
    assert_eq!(permit.pid, 42);
    assert!(matches!(
        peer.retained.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    drop(permit);
    assert!(peer.enter().is_ok(), "authority available after release");
}

#[test]
fn revocation_is_shared_and_permanent() {
    let lease = SignalLease::new(42);
    let peer = lease.clone();
    lease.revoke();
    assert_missing_child(&peer);
    peer.revoke();
    assert_missing_child(&lease);
}

#[test]
fn missing_child_revokes_before_the_permit_is_released() {
    let lease = SignalLease::new(42);
    let peer = lease.clone();
    let mut permit = lease.enter().expect("retained authority entered");
    let error = permit
        .record(Err(io::Error::from_raw_os_error(libc::ECHILD)))
        .expect_err("child loss reported");
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    assert!(!*permit.retained, "child loss revoked under the permit");
    assert!(matches!(
        peer.retained.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    drop(permit);
    assert_missing_child(&peer);
}

#[test]
fn non_child_observation_error_retains_authority() {
    let lease = SignalLease::new(42);
    let mut permit = lease.enter().expect("retained authority entered");
    let error = permit
        .record(Err(io::Error::from_raw_os_error(libc::EIO)))
        .expect_err("observation failure retained");
    assert_eq!(error.raw_os_error(), Some(libc::EIO));
    drop(permit);
    assert!(
        lease.enter().is_ok(),
        "identity survived observation failure"
    );
}

#[test]
fn terminal_observation_does_not_release_authority() {
    let lease = SignalLease::new(42);
    for status in [Status::Exited(3), Status::Signaled(libc::SIGKILL)] {
        let mut permit = lease.enter().expect("retained authority entered");
        assert_eq!(
            permit.record(Ok(Some(status))).expect("status retained"),
            Some(status)
        );
        drop(permit);
        assert!(lease.enter().is_ok(), "terminal leader remains unreaped");
    }
}

#[test]
fn poison_recovery_preserves_revocation() {
    let lease = SignalLease::new(42);
    poison(&lease, false);
    assert_missing_child(&lease);
    lease.revoke();
    assert_missing_child(&lease.clone());
}

#[test]
fn poisoned_retained_authority_can_still_be_revoked() {
    let lease = SignalLease::new(42);
    poison(&lease, true);
    assert!(lease.enter().is_ok(), "retained poison state recovered");
    lease.revoke();
    assert_missing_child(&lease);
}

fn poison(lease: &SignalLease, retained: bool) {
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let mut flag = lease.retained.lock().expect("initial lease lock");
        *flag = retained;
        panic!("controlled lease poison");
    }));
    assert!(panic.is_err());
    assert!(lease.retained.is_poisoned());
}

fn assert_missing_child(lease: &SignalLease) {
    let error = match lease.enter() {
        Ok(_) => panic!("released authority was reacquired"),
        Err(error) => error,
    };
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
}
