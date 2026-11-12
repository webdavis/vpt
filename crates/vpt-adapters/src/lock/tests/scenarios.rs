use super::*;
use std::os::fd::AsRawFd;

pub(crate) fn run(scenario: &str, state: RootDir, ready: fn()) {
    let lock_name = state.path().join("write.lock");
    match scenario {
        "lock_max" => {
            let _held = WriteLock::acquire(&state, Duration::MAX).expect("uncontended lock");
        }
        "lock_zero_self" => {
            let _held = WriteLock::acquire(&state, Duration::ZERO).expect("first lock");
            assert_eq!(
                WriteLock::acquire(&state, Duration::ZERO).err(),
                Some(LockError::Busy)
            );
        }
        "lock_bounded_self" => {
            let _held = WriteLock::acquire(&state, Duration::ZERO).expect("first lock");
            let started = Instant::now();
            assert_eq!(
                WriteLock::acquire(&state, Duration::from_millis(30)).err(),
                Some(LockError::Busy)
            );
            assert!(
                started.elapsed() >= Duration::from_millis(20),
                "contended lock did not wait"
            );
            assert!(
                started.elapsed() < crate::database::WATCHDOG,
                "contended lock exceeded its short wait"
            );
        }
        "lock_drop" => {
            let held = WriteLock::acquire(&state, Duration::ZERO).expect("first lock");
            assert_eq!(
                WriteLock::acquire(&state, Duration::ZERO).err(),
                Some(LockError::Busy)
            );
            drop(held);
            let _next = WriteLock::acquire(&state, Duration::ZERO).expect("released lock");
        }
        "lock_private" => {
            let _held = WriteLock::acquire(&state, Duration::ZERO).expect("created lock");
        }
        "lock_cloexec" => {
            let held = WriteLock::acquire(&state, Duration::ZERO).expect("created lock");
            // SAFETY: held owns this live file descriptor throughout fcntl.
            let flags = unsafe { libc::fcntl(held.file.as_raw_fd(), libc::F_GETFD) };
            assert!(
                flags >= 0,
                "fcntl failed: {}",
                std::io::Error::last_os_error()
            );
            assert_ne!(flags & libc::FD_CLOEXEC, 0);
        }
        "lock_io" => {
            let result = try_lock(-1);
            assert!(
                matches!(&result, Err(LockError::Io(text)) if !text.is_empty()),
                "invalid descriptor must be an I/O error: {result:?}"
            );
        }
        "lock_symlink" | "lock_directory" | "lock_fifo" => {
            assert_eq!(
                WriteLock::acquire(&state, Duration::ZERO).err(),
                Some(LockError::Contained(ContainedError::NotRegular(lock_name)))
            );
        }
        "lock_hardlink" => {
            assert_eq!(
                WriteLock::acquire(&state, Duration::ZERO).err(),
                Some(LockError::Contained(ContainedError::Escape {
                    root: state.path().into(),
                    path: lock_name,
                }))
            );
        }
        "lock_hold" => {
            let _held = WriteLock::acquire(&state, Duration::ZERO).expect("holder lock");
            ready();
        }
        "lock_zero_peer" => {
            assert_eq!(
                WriteLock::acquire(&state, Duration::ZERO).err(),
                Some(LockError::Busy)
            );
        }
        _ => panic!("unknown lock child scenario"),
    }
}
