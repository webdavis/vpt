use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) const PRESSURE_BYTES: usize = 1_048_576;
pub(super) const OUT_MARKER: &[u8] = b"\nVPT-STDOUT\n";
pub(super) const ERR_MARKER: &[u8] = b"\nVPT-STDERR\n";

#[test]
fn child_exchange() {
    let Some(root) = std::env::var_os("VPT_PROCESS_FIXTURE") else {
        return;
    };
    let root = Path::new(&root);
    let started = Instant::now();
    // SAFETY: this post-exec fixture changes only its own TERM disposition.
    unsafe { libc::signal(libc::SIGTERM, libc::SIG_IGN) };
    let code = match exchange(root, started) {
        Ok(()) => 0,
        Err(error) => {
            let _ = std::fs::write(root.join("io-error"), format!("{:?}", error.kind()));
            77
        }
    };
    std::process::exit(code);
}

fn exchange(root: &Path, started: Instant) -> io::Result<()> {
    let mode = std::env::var("VPT_PROCESS_IO_MODE").map_err(|_| io::ErrorKind::InvalidInput)?;
    if mode == "closed-input" {
        // SAFETY: this post-exec child owns its standard input and never uses it afterward.
        if unsafe { libc::close(libc::STDIN_FILENO) } == -1 {
            return Err(io::Error::last_os_error());
        }
        super::lifecycle_fixture::publish_ready(root, b"closed-input")?;
        loop {
            check_bound(root, started);
            std::thread::yield_now();
        }
    }
    let pressure = match mode.as_str() {
        "exact" => false,
        "pressure" => true,
        _ => return Err(io::ErrorKind::InvalidInput.into()),
    };
    let expected: usize = std::env::var("VPT_PROCESS_INPUT_BYTES")
        .map_err(|_| io::ErrorKind::InvalidInput)?
        .parse()
        .map_err(|_| io::ErrorKind::InvalidInput)?;
    if expected > PRESSURE_BYTES {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut input = File::from(std::io::stdin().as_fd().try_clone_to_owned()?);
    let mut output = File::from(std::io::stdout().as_fd().try_clone_to_owned()?);
    let mut error = File::from(std::io::stderr().as_fd().try_clone_to_owned()?);
    nonblocking(&input)?;
    nonblocking(&output)?;
    nonblocking(&error)?;
    let output_bytes = if pressure {
        [OUT_MARKER, &vec![b'o'; PRESSURE_BYTES]].concat()
    } else {
        [OUT_MARKER, b"native-out"].concat()
    };
    let error_bytes = if pressure {
        [ERR_MARKER, &vec![b'e'; PRESSURE_BYTES]].concat()
    } else {
        [ERR_MARKER, b"native-error"].concat()
    };
    let (mut written, mut error_written) = (0, 0);
    let (mut output_blocked, mut error_blocked) = (false, false);
    if pressure {
        while !output_blocked || !error_blocked {
            check_bound(root, started);
            if !output_blocked {
                output_blocked = advance(&mut output, &output_bytes, &mut written)?;
            }
            if !error_blocked {
                error_blocked = advance(&mut error, &error_bytes, &mut error_written)?;
            }
            if written == output_bytes.len() || error_written == error_bytes.len() {
                return Err(io::ErrorKind::InvalidData.into());
            }
            std::thread::yield_now();
        }
    }
    super::lifecycle_fixture::publish_ready(root, b"exchange")?;
    let mut received = Vec::new();
    let mut eof = false;
    while !eof || written < output_bytes.len() || error_written < error_bytes.len() {
        check_bound(root, started);
        advance(&mut output, &output_bytes, &mut written)?;
        advance(&mut error, &error_bytes, &mut error_written)?;
        if !eof {
            let mut buffer = [0; 8192];
            match input.read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(count) => {
                    if received.len() + count > expected {
                        return Err(io::ErrorKind::InvalidData.into());
                    }
                    received.extend_from_slice(&buffer[..count]);
                }
                Err(error) if transient(&error) => {}
                Err(error) => return Err(error),
            }
        }
        std::thread::yield_now();
    }
    if received.len() != expected {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    std::fs::write(root.join("input"), &received)?;
    std::fs::write(
        root.join("counts"),
        format!(
            "{} {written} {error_written} {output_blocked} {error_blocked} {eof}",
            received.len()
        ),
    )?;
    Ok(())
}

fn advance(writer: &mut File, bytes: &[u8], position: &mut usize) -> io::Result<bool> {
    if *position == bytes.len() {
        return Ok(false);
    }
    match writer.write(&bytes[*position..(*position + 8192).min(bytes.len())]) {
        Ok(0) => Err(io::ErrorKind::WriteZero.into()),
        Ok(count) => {
            *position += count;
            Ok(false)
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(false),
        Err(error) => Err(error),
    }
}

fn nonblocking(file: &File) -> io::Result<()> {
    // SAFETY: file retains this live owned descriptor throughout both flag operations.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: only status flags of the same retained descriptor change.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn transient(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}

fn check_bound(root: &Path, started: Instant) {
    if started.elapsed() >= Duration::from_millis(600) {
        std::process::exit(
            if std::fs::write(root.join("fallback"), b"expired").is_ok() {
                75
            } else {
                76
            },
        );
    }
}
