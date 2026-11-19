use serde_json::json;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

const INPUT_BOUND: Duration = Duration::from_millis(300);

enum InputClock {
    Real(Instant),
    Controlled(Duration),
}

impl InputClock {
    fn elapsed(&self) -> Duration {
        match self {
            Self::Real(started) => started.elapsed(),
            Self::Controlled(elapsed) => *elapsed,
        }
    }

    fn pending(&mut self) -> bool {
        if let Self::Controlled(elapsed) = self
            && *elapsed == Duration::ZERO
        {
            *elapsed = INPUT_BOUND;
            return true;
        }
        false
    }
}

pub(super) fn run(args: &[String]) -> i32 {
    let wanted = std::env::var("VPT_FAKE_INPUT_LEN")
        .ok()
        .and_then(|value| value.parse::<usize>().ok());
    let Some(wanted) = wanted.filter(|wanted| *wanted <= 65_536) else {
        return 2;
    };
    let input = match read_input(wanted) {
        Ok(bytes) => bytes,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "{message}");
            return 75;
        }
    };
    let mut environment = serde_json::Map::new();
    for key in [
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
        "TMPDIR",
        "VPT_CONFIG",
        "PATH",
        "VPT_FAKE_TRASH",
        "VPT_FAKE_LOG",
    ] {
        environment.insert(key.into(), json!(std::env::var(key).ok()));
    }
    let Ok(input) = String::from_utf8(input) else {
        return 75;
    };
    let document = json!({"argv": args, "stdin": input, "environment": environment});
    let mut output = io::stdout().lock();
    if serde_json::to_writer(&mut output, &document).is_err()
        || writeln!(output).is_err()
        || writeln!(io::stderr().lock(), "owned stderr marker").is_err()
    {
        return 1;
    }
    0
}

fn read_input(wanted: usize) -> Result<Vec<u8>, &'static str> {
    let mut clock = if std::env::var("VPT_FAKE_EXPIRE_INPUT_ON_PENDING").as_deref() == Ok("1") {
        InputClock::Controlled(Duration::ZERO)
    } else {
        InputClock::Real(Instant::now())
    };
    let mut input = io::stdin().lock();
    // SAFETY: stdin is a live fixture descriptor; only its status flags change.
    let nonblocking = unsafe {
        let flags = libc::fcntl(input.as_raw_fd(), libc::F_GETFL);
        if flags == -1 {
            -1
        } else {
            libc::fcntl(input.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK)
        }
    };
    if nonblocking == -1 {
        return Err("fixture input flags");
    }
    let mut bytes = Vec::with_capacity(wanted);
    let mut buffer = [0; 8192];
    while bytes.len() < wanted {
        if clock.elapsed() >= INPUT_BOUND {
            return Err("fixture input deadline");
        }
        let count = buffer.len().min(wanted - bytes.len());
        match input.read(&mut buffer[..count]) {
            Ok(0) => return Err("fixture input incomplete"),
            Ok(count) => bytes.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if clock.pending() {
                    let _ = writeln!(io::stderr().lock(), "fixture input pending");
                }
                let mut descriptor = libc::pollfd {
                    fd: input.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                // SAFETY: the one initialized entry borrows the live stdin descriptor.
                if unsafe { libc::poll(&mut descriptor, 1, 1) } == -1
                    && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
                {
                    return Err("fixture input poll");
                }
            }
            Err(_) => return Err("fixture input read"),
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controlled_input_clock_expires_only_after_pending_input() {
        let mut clock = InputClock::Controlled(Duration::ZERO);
        assert_eq!(clock.elapsed(), Duration::ZERO);
        assert!(
            clock.pending(),
            "first pending read must advance the controlled clock"
        );
        assert_eq!(clock.elapsed(), INPUT_BOUND);
        assert!(!clock.pending(), "pending transition is emitted only once");
    }
}
