mod movie;
use crate::time::UtcInstant;

const MPEG4_EPOCH_OFFSET: i64 = 2_082_844_800;

#[derive(Debug, PartialEq, Eq)]
pub struct ReadFailure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Container {
    pub creation_time: UtcInstant,
    pub duration_secs: u64,
    duration_ticks: u64,
    timescale: u32,
}

impl Container {
    pub fn recorded_creation_time(&self) -> Option<UtcInstant> {
        (self.creation_time.secs != -MPEG4_EPOCH_OFFSET).then_some(self.creation_time)
    }

    pub fn precise_duration_secs(&self) -> f64 {
        self.duration_ticks as f64 / f64::from(self.timescale)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContainerError {
    PastEnd { offset: u64 },
    InvalidLength { offset: u64 },
    Overflow,
    MissingMoov,
    MissingMvhd,
    InvalidMvhd,
    Unreadable,
}

struct Header {
    kind: [u8; 4],
    length: u64,
    header_len: u64,
}

pub fn inspect(
    len: u64,
    mut read: impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure>,
) -> Result<Container, ContainerError> {
    let mut offset = 0;
    let mut moov = None;
    while offset < len {
        let header = read_header(&mut read, offset, len, len)?;
        if &header.kind == b"moov" {
            moov = Some((offset + header.header_len, offset + header.length));
        }
        offset = offset
            .checked_add(header.length)
            .ok_or(ContainerError::Overflow)?;
    }

    let (start, end) = moov.ok_or(ContainerError::MissingMoov)?;
    let mut child = start;
    while child < end {
        let header = read_header(&mut read, child, len, end)?;
        if &header.kind == b"mvhd" {
            return movie::read(
                &mut read,
                child + header.header_len,
                header.length - header.header_len,
            );
        }
        child = child
            .checked_add(header.length)
            .ok_or(ContainerError::Overflow)?;
    }
    Err(ContainerError::MissingMvhd)
}

fn read_header(
    read: &mut impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure>,
    offset: u64,
    file_end: u64,
    parent_end: u64,
) -> Result<Header, ContainerError> {
    if offset.checked_add(8).ok_or(ContainerError::Overflow)? > parent_end {
        return Err(ContainerError::PastEnd { offset });
    }
    let mut head = [0u8; 8];
    read(offset, &mut head).map_err(|_| ContainerError::Unreadable)?;
    let size = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let kind = [head[4], head[5], head[6], head[7]];
    let (length, header_len) = match size {
        0 => (file_end - offset, 8),
        1 => {
            if offset.checked_add(16).ok_or(ContainerError::Overflow)? > parent_end {
                return Err(ContainerError::PastEnd { offset });
            }
            let mut large = [0u8; 8];
            read(offset + 8, &mut large).map_err(|_| ContainerError::Unreadable)?;
            (u64::from_be_bytes(large), 16)
        }
        n => (u64::from(n), 8),
    };
    if length < header_len {
        return Err(ContainerError::InvalidLength { offset });
    }
    if offset.checked_add(length).ok_or(ContainerError::Overflow)? > parent_end {
        return Err(ContainerError::PastEnd { offset });
    }
    Ok(Header {
        kind,
        length,
        header_len,
    })
}

#[cfg(test)]
mod tests;
