use super::Status;

pub(super) fn only_terminal_leader(
    leader: libc::pid_t,
    terminal: Option<Status>,
    count: libc::c_int,
    members: &[libc::pid_t; 2],
) -> bool {
    leader > 1
        && matches!(terminal, Some(Status::Exited(_) | Status::Signaled(_)))
        && count == 1
        && members[0] == leader
}

#[cfg(test)]
mod tests;
