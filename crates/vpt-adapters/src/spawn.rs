mod group;
mod io;
mod lease;
mod membership;
mod process;
mod reap;
mod runtime;

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const OUTPUT_LIMIT: usize = 65_536;
pub const GRACE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Exited(i32),
    Signaled(i32),
    DeadlineExceeded,
    Interrupted,
}

pub struct Outcome {
    pub status: Status,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub group: libc::pid_t,
}

impl fmt::Debug for Outcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Outcome")
            .field("status", &self.status)
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr.len())
            .field("group", &self.group)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnError {
    NotFound(String),
    Io(String),
}

pub fn run(
    argv: &[String],
    stdin: &[u8],
    deadline: Duration,
    output_limit: usize,
) -> Result<Outcome, SpawnError> {
    run_with_env(argv, stdin, deadline, output_limit, &[])
}

pub fn run_with_env(
    argv: &[String],
    stdin: &[u8],
    deadline: Duration,
    output_limit: usize,
    env: &[(&str, &str)],
) -> Result<Outcome, SpawnError> {
    let (program, arguments) = argv
        .split_first()
        .ok_or_else(|| SpawnError::Io("empty argv".into()))?;
    let mut command = std::process::Command::new(program);
    command.args(arguments).envs(env.iter().copied());
    runtime::execute(
        command,
        stdin,
        deadline,
        output_limit,
        production_controls(),
    )
}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

pub fn install_interrupt_handlers() {
    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
        // SAFETY: the handler only stores into a lock-free atomic flag.
        unsafe { libc::signal(signal, on_signal as *const () as libc::sighandler_t) };
    }
}

fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

type ProductionControls = runtime::Controls<
    fn() -> bool,
    fn() -> Instant,
    fn(Duration),
    fn(lease::SignalLease, &mut runtime::PipeSlots, &mut Option<()>) -> std::io::Result<()>,
    fn(std::process::ExitStatus),
>;

fn production_controls() -> ProductionControls {
    runtime::Controls {
        interrupted,
        now: Instant::now,
        wait: std::thread::sleep,
        grace: GRACE,
        observed: |_, _, _| Ok(()),
        reaped: |_| {},
    }
}

#[cfg(test)]
mod tests;
