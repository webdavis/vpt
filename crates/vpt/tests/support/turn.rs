use std::sync::{Mutex, MutexGuard, PoisonError};

static LANE: Mutex<()> = Mutex::new(());

/// Lets one timed command run at a time, so its budget measures its own work
/// and not the disk synchronizations of the tests running beside it.
pub fn turn() -> MutexGuard<'static, ()> {
    LANE.lock().unwrap_or_else(PoisonError::into_inner)
}
