use std::io::{self, Read, Write};

pub(super) struct Capture {
    pub bytes: Vec<u8>,
    pub finished: bool,
    limit: usize,
}

impl Capture {
    pub fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            finished: false,
            limit,
        }
    }

    pub fn drain(&mut self, reader: &mut impl Read) -> io::Result<bool> {
        if self.finished {
            return Ok(false);
        }
        let mut buffer = [0; 8192];
        let mut progress = false;
        for _ in 0..8 {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    self.finished = true;
                    return Ok(true);
                }
                Ok(count) => {
                    let retained = self.limit.saturating_sub(self.bytes.len()).min(count);
                    self.bytes.extend_from_slice(&buffer[..retained]);
                    progress = true;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(progress)
    }
}

pub(super) struct Input<'a> {
    bytes: &'a [u8],
}

impl<'a> Input<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn write(&mut self, writer: &mut impl Write) -> io::Result<bool> {
        for _ in 0..8 {
            if self.bytes.is_empty() {
                return Ok(true);
            }
            match writer.write(&self.bytes[..self.bytes.len().min(8192)]) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => self.bytes = &self.bytes[count..],
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        Ok(self.bytes.is_empty())
    }
}

#[cfg(test)]
mod tests;
