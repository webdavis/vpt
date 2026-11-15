use std::io::{self, Read};
use std::os::fd::AsRawFd;

pub(super) struct Capture<T> {
    pipe: Option<T>,
    pub(super) bytes: Vec<u8>,
}

impl<T: Read + AsRawFd> Capture<T> {
    pub(super) fn new(pipe: T) -> io::Result<Self> {
        nonblocking(&pipe)?;
        Ok(Self {
            pipe: Some(pipe),
            bytes: Vec::new(),
        })
    }

    pub(super) fn finished(&self) -> bool {
        self.pipe.is_none()
    }

    pub(super) fn drain(&mut self) -> io::Result<()> {
        let Some(pipe) = &mut self.pipe else {
            return Ok(());
        };
        let mut buffer = [0; 8192];
        for _ in 0..8 {
            match pipe.read(&mut buffer) {
                Ok(0) => {
                    self.pipe = None;
                    break;
                }
                Ok(count) => {
                    if self.bytes.len() + count > 65536 {
                        return Err(io::Error::other("child output exceeds 64 KiB"));
                    }
                    self.bytes.extend_from_slice(&buffer[..count]);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

pub(super) fn nonblocking(file: &impl AsRawFd) -> io::Result<()> {
    // SAFETY: the borrowed live descriptor retains its existing flags.
    let result = unsafe {
        let flags = libc::fcntl(file.as_raw_fd(), libc::F_GETFL);
        if flags == -1 {
            -1
        } else {
            libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK)
        }
    };
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
