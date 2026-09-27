use crate::support::process::{Identity, cleanup_observation, decode_status};
use std::cell::Cell;
use std::io;
use std::os::unix::process::ExitStatusExt;

#[test]
fn normal_status_preserves_exit_value() {
    for code in [0, 23, 255] {
        let status = decode_status(libc::CLD_EXITED, code).expect("normal exit status");
        assert_eq!(status.code(), Some(code));
        assert_eq!(status.signal(), None);
        assert!(!status.core_dumped());
    }
}

#[test]
fn signal_status_preserves_signal_without_core() {
    let status = decode_status(libc::CLD_KILLED, libc::SIGTERM).expect("signal exit status");
    assert_eq!(status.code(), None);
    assert_eq!(status.signal(), Some(libc::SIGTERM));
    assert!(!status.core_dumped());
}

#[test]
fn dumped_status_preserves_signal_and_core() {
    let status = decode_status(libc::CLD_DUMPED, libc::SIGABRT).expect("core flag status");
    assert_eq!(status.code(), None);
    assert_eq!(status.signal(), Some(libc::SIGABRT));
    assert!(status.core_dumped());
}

#[test]
fn unknown_wait_events_are_rejected() {
    for event in [0, libc::CLD_STOPPED, libc::CLD_TRAPPED, libc::CLD_CONTINUED] {
        let error = decode_status(event, 23).expect_err("only terminal events are accepted");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}

#[test]
fn echild_prevents_every_later_observation() {
    let mut identity = Identity::default();
    let calls = Cell::new(0);
    let first = identity.observe_with(|| {
        calls.set(calls.get() + 1);
        Err(io::Error::from_raw_os_error(libc::ECHILD))
    });
    let later = identity.observe_with(|| {
        calls.set(calls.get() + 1);
        Ok(None)
    });
    assert_eq!(first.unwrap_err().raw_os_error(), Some(libc::ECHILD));
    assert_eq!(
        later
            .expect_err("lost identity cannot be observed again")
            .raw_os_error(),
        Some(libc::ECHILD)
    );
    assert_eq!(calls.get(), 1, "lost identity invoked another observation");
}

#[test]
fn interruption_preserves_observation_authority() {
    let mut identity = Identity::default();
    let first = identity.observe_with(|| Err(io::Error::from_raw_os_error(libc::EINTR)));
    let later = identity.observe_with(|| Ok(None));
    assert_eq!(first.unwrap_err().raw_os_error(), Some(libc::EINTR));
    assert_eq!(later.unwrap(), None);
}

#[test]
fn observation_error_preserves_cleanup_authority() {
    let mut recorded = None;
    let result = cleanup_observation(Err(io::Error::from_raw_os_error(libc::EIO)), &mut recorded);
    assert!(!result.expect("owned cleanup must continue after an observation error"));
    assert_eq!(recorded.unwrap().raw_os_error(), Some(libc::EIO));
}

#[test]
fn interrupted_observation_preserves_cleanup_authority() {
    let mut recorded = None;
    let result = cleanup_observation(
        Err(io::Error::from_raw_os_error(libc::EINTR)),
        &mut recorded,
    );
    assert!(!result.expect("interrupted observation must allow final cleanup"));
    assert_eq!(recorded.unwrap().raw_os_error(), Some(libc::EINTR));
}

#[test]
fn later_observation_preserves_the_first_cleanup_error() {
    let mut recorded = Some(io::Error::from_raw_os_error(libc::EIO));
    let result = cleanup_observation(
        Err(io::Error::from_raw_os_error(libc::EINTR)),
        &mut recorded,
    );
    assert!(!result.expect("later errors must allow final cleanup"));
    assert!(cleanup_observation(Ok(None), &mut recorded).unwrap());
    assert_eq!(recorded.unwrap().raw_os_error(), Some(libc::EIO));
}

#[test]
fn lost_identity_refuses_cleanup_authority() {
    let mut recorded = None;
    let result = cleanup_observation(
        Err(io::Error::from_raw_os_error(libc::ECHILD)),
        &mut recorded,
    );
    assert_eq!(result.unwrap_err().raw_os_error(), Some(libc::ECHILD));
    assert!(recorded.is_none());
}
