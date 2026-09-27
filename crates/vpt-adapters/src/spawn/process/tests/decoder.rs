use super::super::{Status, decode_status};
use std::io;

#[test]
fn normal_exit_preserves_the_reported_exit_value() {
    for value in [0, 3, 255] {
        assert_eq!(
            decode_status(libc::CLD_EXITED, value).expect("terminal exit decoded"),
            Status::Exited(value)
        );
    }
}

#[test]
fn killed_child_preserves_the_terminating_signal() {
    for signal in [libc::SIGTERM, libc::SIGKILL] {
        assert_eq!(
            decode_status(libc::CLD_KILLED, signal).expect("killed status decoded"),
            Status::Signaled(signal)
        );
    }
}

#[test]
fn dumped_child_preserves_the_terminating_signal() {
    for signal in [libc::SIGSEGV, libc::SIGABRT] {
        assert_eq!(
            decode_status(libc::CLD_DUMPED, signal).expect("dumped status decoded"),
            Status::Signaled(signal)
        );
    }
}

#[test]
fn nonterminal_status_codes_are_not_completion() {
    for code in [libc::CLD_TRAPPED, libc::CLD_STOPPED, libc::CLD_CONTINUED] {
        assert_eq!(
            decode_status(code, libc::SIGSTOP)
                .expect_err("nonterminal status rejected")
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}

#[test]
fn unknown_status_codes_are_not_completion() {
    for code in [libc::c_int::MIN, -1, 0, libc::c_int::MAX] {
        assert_eq!(
            decode_status(code, 0)
                .expect_err("unknown status rejected")
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
