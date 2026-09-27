use super::*;
use std::cell::Cell;
use std::os::unix::process::ExitStatusExt;

#[test]
fn pending_and_interrupted_waits_require_exact_status() {
    let (result, polls) = scripted(
        None,
        vec![Ok(None), failure(libc::EINTR), Ok(Some(exit(3)))],
    );
    let reaped = result.expect("exact child reaped");
    assert_eq!(reaped.status.code(), Some(3));
    assert!(
        reaped.failure.is_none(),
        "interruption is not a genuine failure"
    );
    assert_eq!(polls, 3);
}

#[test]
fn observation_failure_is_retained_after_successful_exact_reap() {
    let (result, polls) = scripted(
        Some(io::Error::from_raw_os_error(libc::EIO)),
        vec![Ok(None), Ok(Some(exit(0)))],
    );
    let reaped = result.expect("observation error still permits exact reap");
    assert_eq!(reaped.status.code(), Some(0));
    assert_eq!(
        reaped
            .failure
            .expect("first failure retained")
            .raw_os_error(),
        Some(libc::EIO)
    );
    assert_eq!(polls, 2);
}

#[test]
fn wait_failure_keeps_polling_until_exact_status() {
    let (result, polls) = scripted(
        None,
        vec![
            failure(libc::EIO),
            Ok(None),
            Ok(Some(ExitStatus::from_raw(libc::SIGKILL))),
        ],
    );
    let reaped = result.expect("wait recovered and reaped exact child");
    assert_eq!(reaped.status.signal(), Some(libc::SIGKILL));
    assert_eq!(
        reaped
            .failure
            .expect("wait failure retained")
            .raw_os_error(),
        Some(libc::EIO)
    );
    assert_eq!(polls, 3);
}

#[test]
fn first_failure_is_not_replaced_by_later_wait_failure() {
    let (result, polls) = scripted(
        Some(io::Error::from_raw_os_error(libc::EIO)),
        vec![failure(libc::EACCES), Ok(Some(exit(0)))],
    );
    let reaped = result.expect("all errors followed by exact reap");
    assert_eq!(
        reaped
            .failure
            .expect("first failure retained")
            .raw_os_error(),
        Some(libc::EIO)
    );
    assert_eq!(polls, 2);
}

#[test]
fn missing_child_ends_polling_without_a_reaping_result() {
    let (result, polls) = scripted(None, vec![failure(libc::ECHILD), Ok(Some(exit(0)))]);
    assert_missing_child(result);
    assert_eq!(polls, 1, "no polling after permanent identity loss");
}

#[test]
fn missing_child_is_reported_even_after_an_observation_failure() {
    let (result, polls) = scripted(
        Some(io::Error::from_raw_os_error(libc::EIO)),
        vec![failure(libc::ECHILD)],
    );
    assert_missing_child(result);
    assert_eq!(polls, 1);
}

fn scripted(
    first_failure: Option<io::Error>,
    steps: Vec<io::Result<Option<ExitStatus>>>,
) -> (io::Result<Reaped>, usize) {
    let bound = steps.len();
    let mut steps = steps.into_iter();
    let polls = Cell::new(0);
    let waits = Cell::new(0);
    let result = finish(
        first_failure,
        || {
            polls.set(polls.get() + 1);
            steps.next().expect("finite scripted poll bound")
        },
        || {
            waits.set(waits.get() + 1);
            assert!(waits.get() <= bound, "finite scripted wait bound");
        },
    );
    (result, polls.get())
}

fn exit(code: i32) -> ExitStatus {
    ExitStatus::from_raw(code << 8)
}

fn failure(errno: i32) -> io::Result<Option<ExitStatus>> {
    Err(io::Error::from_raw_os_error(errno))
}

fn assert_missing_child(result: io::Result<Reaped>) {
    let error = match result {
        Ok(_) => panic!("identity loss cannot claim exact reaping"),
        Err(error) => error,
    };
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
}
