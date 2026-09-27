use std::process::ExitStatus;
use std::time::{Duration, Instant};

pub(super) struct Diagnostic {
    started: Instant,
    child: u32,
    spawn_return: Option<Duration>,
    driver: Option<Duration>,
    stdout_drain: Option<Duration>,
    stderr_drain: Option<Duration>,
    try_wait: Option<Duration>,
    child_exit: Option<Duration>,
    stdout_eof: Option<Duration>,
    stderr_eof: Option<Duration>,
    deadline: Option<Duration>,
    cleanup: Option<Duration>,
    poll_completed: Instant,
    poll_gap: Duration,
    active_step: (&'static str, Duration),
    status: Option<ExitStatus>,
}

impl Diagnostic {
    pub(super) fn new(started: Instant, child: u32, spawn_return: Option<Instant>) -> Self {
        let returned = Instant::now();
        Self {
            started,
            child,
            spawn_return: spawn_return.map(|time| time.duration_since(started)),
            driver: None,
            stdout_drain: None,
            stderr_drain: None,
            try_wait: None,
            child_exit: None,
            stdout_eof: None,
            stderr_eof: None,
            deadline: None,
            cleanup: None,
            poll_completed: returned,
            poll_gap: Duration::ZERO,
            active_step: ("none", Duration::ZERO),
            status: None,
        }
    }

    pub(super) fn poll_started(&mut self, now: Instant) {
        self.poll_gap = self.poll_gap.max(now.duration_since(self.poll_completed));
    }

    pub(super) fn step(&mut self, phase: &'static str, from: Instant, to: Instant) {
        let offset = to.duration_since(self.started);
        let duration = to.duration_since(from);
        if duration > self.active_step.1 {
            self.active_step = (phase, duration);
        }
        match phase {
            "driver" => self.driver = Some(offset),
            "stdout_drain" => self.stdout_drain = Some(offset),
            "stderr_drain" => self.stderr_drain = Some(offset),
            "try_wait" => self.try_wait = Some(offset),
            _ => unreachable!("known polling phase"),
        }
    }

    pub(super) fn stdout_eof(&mut self, now: Instant) {
        self.stdout_eof
            .get_or_insert(now.duration_since(self.started));
    }

    pub(super) fn stderr_eof(&mut self, now: Instant) {
        self.stderr_eof
            .get_or_insert(now.duration_since(self.started));
    }

    pub(super) fn child_exit(&mut self, now: Instant, status: ExitStatus) {
        if self.child_exit.is_none() {
            self.child_exit = Some(now.duration_since(self.started));
            self.status = Some(status);
        }
    }

    pub(super) fn deadline(&mut self, now: Instant) {
        self.deadline = Some(now.duration_since(self.started));
    }

    pub(super) fn poll_completed(&mut self, now: Instant) {
        self.poll_completed = now;
    }

    pub(super) fn cleanup(&mut self, now: Instant) {
        self.cleanup = Some(now.duration_since(self.started));
    }

    pub(super) fn render(
        &self,
        stdout: &[u8],
        stderr: &[u8],
        stdout_closed: bool,
        stderr_closed: bool,
    ) -> String {
        let offset = |value: Option<Duration>| {
            value.map_or_else(|| "none".to_owned(), |duration| format!("{duration:?}"))
        };
        let preview = |bytes: &[u8]| bytes[..bytes.len().min(64)].escape_ascii().to_string();
        format!(
            "child={} status={:?} spawn_return={} driver={} stdout_drain={} stderr_drain={} try_wait={} child_exit={} stdout_eof={} stderr_eof={} deadline={} cleanup={} poll_gap={:?} active_step={}:{:?} stdout_closed={} stderr_closed={} stdout_bytes={} stderr_bytes={} stdout_preview={} stderr_preview={}",
            self.child,
            self.status,
            offset(self.spawn_return),
            offset(self.driver),
            offset(self.stdout_drain),
            offset(self.stderr_drain),
            offset(self.try_wait),
            offset(self.child_exit),
            offset(self.stdout_eof),
            offset(self.stderr_eof),
            offset(self.deadline),
            offset(self.cleanup),
            self.poll_gap,
            self.active_step.0,
            self.active_step.1,
            stdout_closed,
            stderr_closed,
            stdout.len(),
            stderr.len(),
            preview(stdout),
            preview(stderr),
        )
    }
}
