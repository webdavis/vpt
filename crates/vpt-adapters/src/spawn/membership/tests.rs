use super::*;

const LEADER: libc::pid_t = 42;

#[test]
fn one_exact_owned_terminal_leader_can_finish_grace() {
    for terminal in [Status::Exited(0), Status::Exited(3), Status::Signaled(9)] {
        assert!(only_terminal_leader(
            LEADER,
            Some(terminal),
            1,
            &[LEADER, 0]
        ));
    }
}

#[test]
fn a_running_or_cancelled_status_cannot_finish_grace() {
    for terminal in [
        None,
        Some(Status::Interrupted),
        Some(Status::DeadlineExceeded),
    ] {
        assert!(!only_terminal_leader(LEADER, terminal, 1, &[LEADER, 0]));
    }
}

#[test]
fn zero_or_negative_listing_results_cannot_finish_grace() {
    for count in [i32::MIN, -1, 0] {
        assert!(!only_terminal_leader(
            LEADER,
            Some(Status::Exited(0)),
            count,
            &[LEADER, 0],
        ));
    }
}

#[test]
fn full_or_oversized_listing_results_cannot_finish_grace() {
    for count in [2, 3, i32::MAX] {
        assert!(!only_terminal_leader(
            LEADER,
            Some(Status::Exited(0)),
            count,
            &[LEADER, LEADER + 1],
        ));
    }
}

#[test]
fn mismatched_or_invalid_members_cannot_finish_grace() {
    for member in [i32::MIN, -1, 0, 1, LEADER + 1] {
        assert!(!only_terminal_leader(
            LEADER,
            Some(Status::Exited(0)),
            1,
            &[member, LEADER],
        ));
    }
}

#[test]
fn invalid_owned_identity_cannot_finish_grace() {
    for leader in [i32::MIN, -1, 0, 1] {
        assert!(!only_terminal_leader(
            leader,
            Some(Status::Exited(0)),
            1,
            &[leader, 0],
        ));
    }
}

#[test]
fn an_unreturned_slot_does_not_change_the_complete_one_member_result() {
    assert!(only_terminal_leader(
        LEADER,
        Some(Status::Exited(0)),
        1,
        &[LEADER, LEADER + 1],
    ));
}
