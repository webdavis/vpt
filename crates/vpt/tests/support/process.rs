use std::process::Child;
use std::time::{Duration, Instant};

pub(super) struct ProcessGroup {
    pub child: Child,
    deadline: Instant,
}

impl ProcessGroup {
    pub fn new(child: Child, deadline: Instant) -> Self {
        Self { child, deadline }
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        let pid = i32::try_from(self.child.id()).expect("child pid fits pid_t");
        // SAFETY: spawn creates the owned process group with the child's PID.
        unsafe { libc::killpg(pid, libc::SIGTERM) };
        let grace = (Instant::now() + Duration::from_millis(50)).min(self.deadline);
        while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < grace {
            std::thread::sleep(Duration::from_millis(1));
        }
        // SAFETY: the same owned group includes any surviving fixture descendants.
        unsafe { libc::killpg(pid, libc::SIGKILL) };
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < self.deadline => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                result => panic!("owned child was not reaped before cleanup deadline: {result:?}"),
            }
        }
    }
}
