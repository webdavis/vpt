use std::io::{self, Read};
use std::os::fd::AsRawFd;

pub(super) struct Capture<T> {
    pipe: Option<T>,
    pub bytes: Vec<u8>,
}

impl<T: Read + AsRawFd> Capture<T> {
    pub fn new(pipe: Option<T>) -> Self {
        if let Some(pipe) = &pipe {
            nonblocking(pipe).expect("nonblocking output");
        }
        Self {
            pipe,
            bytes: Vec::new(),
        }
    }

    pub fn finished(&self) -> bool {
        self.pipe.is_none()
    }

    pub fn drain(&mut self) -> io::Result<()> {
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
                Ok(count) => self.bytes.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

pub(crate) fn nonblocking(file: &impl AsRawFd) -> io::Result<()> {
    // SAFETY: the live descriptor is borrowed; fcntl changes only its status flags.
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
