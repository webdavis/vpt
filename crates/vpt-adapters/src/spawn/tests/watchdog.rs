use super::super::lease::SignalLease;
use super::peers::Peer;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(super) const WATCHDOG: Duration = Duration::from_millis(750);

pub(super) struct Registration {
    stop: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
    active: Rc<Cell<bool>>,
    failed: Arc<AtomicBool>,
    peer: Option<Peer>,
}

impl Registration {
    pub fn new(
        lease: SignalLease,
        started: Instant,
        fired: Arc<AtomicBool>,
        active: Rc<Cell<bool>>,
        failed: Arc<AtomicBool>,
    ) -> std::io::Result<Self> {
        assert!(lease.group() > 1, "owned child group");
        let (stop, receive) = mpsc::channel();
        let worker = std::thread::Builder::new().spawn(move || {
            let remaining = WATCHDOG.saturating_sub(started.elapsed());
            if receive.recv_timeout(remaining) == Err(mpsc::RecvTimeoutError::Timeout) {
                fired.store(true, Ordering::SeqCst);
                let _ = lease.signal(libc::SIGKILL);
            }
        })?;
        active.set(true);
        Ok(Self {
            stop: Some(stop),
            worker: Some(worker),
            active,
            failed,
            peer: None,
        })
    }

    pub fn attach_peer(&mut self, peer: Peer, group: libc::pid_t) -> std::io::Result<()> {
        self.peer = Some(peer);
        self.peer
            .as_mut()
            .expect("installed owned peer")
            .start(group)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take()
            && worker.join().is_err()
        {
            self.failed.store(true, Ordering::SeqCst);
        }
        if let Some(mut peer) = self.peer.take()
            && !matches!(peer.finish(), Ok(true))
        {
            self.failed.store(true, Ordering::SeqCst);
        }
        self.active.set(false);
    }
}
