use super::{fixture_child::FixtureChild, reaper::fixture_command, watchdog::WATCHDOG};
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;

enum ReaperState {
    AwaitingGroup,
    OwnsParticipant,
    Reaped,
}

pub(super) struct Peer {
    child: FixtureChild,
    state: ReaperState,
    started: Instant,
}

impl Peer {
    pub fn new(root: &Path, output: OwnedFd, closed: bool, started: Instant) -> io::Result<Self> {
        let mut command = fixture_command(root, "cooperative_reaper");
        command
            .process_group(0)
            .env("VPT_PROCESS_CLOSED_PIPES", if closed { "1" } else { "0" })
            .stdin(Stdio::piped())
            .stdout(output)
            .stderr(Stdio::null());
        let child = command.spawn()?;
        Ok(Self {
            child: FixtureChild::new(child),
            state: ReaperState::AwaitingGroup,
            started,
        })
    }

    pub fn start(&mut self, group: libc::pid_t) -> io::Result<()> {
        if !matches!(self.state, ReaperState::AwaitingGroup) {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        assert!(
            group > 1,
            "executor group registered before participant launch"
        );
        self.state = ReaperState::OwnsParticipant;
        let input = self
            .child
            .input()
            .as_mut()
            .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?;
        writeln!(input, "{group}")
    }

    pub fn finish(&mut self) -> io::Result<bool> {
        if matches!(self.state, ReaperState::Reaped) {
            return Ok(true);
        }
        if matches!(self.state, ReaperState::AwaitingGroup) {
            let _ = self.child.kill();
        } else {
            while self.started.elapsed() < WATCHDOG {
                match self.child.terminal() {
                    Ok(true) => {
                        self.state = ReaperState::Reaped;
                        return self.child.wait().map(|status| status.success());
                    }
                    Ok(false) => std::thread::yield_now(),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        }
        drop(self.child.input().take());
        self.state = ReaperState::Reaped;
        self.child.wait().map(|_| false)
    }

    pub fn pid(&self) -> libc::pid_t {
        self.child.pid()
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
