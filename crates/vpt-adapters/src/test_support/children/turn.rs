use std::sync::{Mutex, MutexGuard, PoisonError};

static LANE: Mutex<()> = Mutex::new(());

/// Lets one timed child run at a time, so its budget measures its own work
/// and not the disk synchronizations of the tests running beside it.
pub(crate) fn turn() -> MutexGuard<'static, ()> {
    LANE.lock().unwrap_or_else(PoisonError::into_inner)
}
