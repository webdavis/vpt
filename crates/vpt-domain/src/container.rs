use crate::time::UtcInstant;

#[derive(Debug, PartialEq, Eq)]
pub struct ReadFailure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Container {
    pub creation_time: UtcInstant,
    pub duration_secs: u64,
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
            return read_mvhd(
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

fn read_mvhd(
    read: &mut impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure>,
    body: u64,
    body_len: u64,
) -> Result<Container, ContainerError> {
    if body_len == 0 {
        return Err(ContainerError::InvalidMvhd);
    }
    let mut version = [0u8; 1];
    read(body, &mut version).map_err(|_| ContainerError::Unreadable)?;
    let (creation, timescale, duration, unknown) = match version[0] {
        0 => {
            if body_len < 20 {
                return Err(ContainerError::InvalidMvhd);
            }
            let mut fields = [0u8; 16];
            read(body + 4, &mut fields).map_err(|_| ContainerError::Unreadable)?;
            let creation = u32::from_be_bytes(
                fields[0..4]
                    .try_into()
                    .map_err(|_| ContainerError::InvalidMvhd)?,
            );
            let timescale = u32::from_be_bytes(
                fields[8..12]
                    .try_into()
                    .map_err(|_| ContainerError::InvalidMvhd)?,
            );
            let duration = u32::from_be_bytes(
                fields[12..16]
                    .try_into()
                    .map_err(|_| ContainerError::InvalidMvhd)?,
            );
            (
                u64::from(creation),
                timescale,
                u64::from(duration),
                duration == u32::MAX,
            )
        }
        1 => {
            if body_len < 32 {
                return Err(ContainerError::InvalidMvhd);
            }
            let mut fields = [0u8; 28];
            read(body + 4, &mut fields).map_err(|_| ContainerError::Unreadable)?;
            let creation = u64::from_be_bytes(
                fields[0..8]
                    .try_into()
                    .map_err(|_| ContainerError::InvalidMvhd)?,
            );
            let timescale = u32::from_be_bytes(
                fields[16..20]
                    .try_into()
                    .map_err(|_| ContainerError::InvalidMvhd)?,
            );
            let duration = u64::from_be_bytes(
                fields[20..28]
                    .try_into()
                    .map_err(|_| ContainerError::InvalidMvhd)?,
            );
            (creation, timescale, duration, duration == u64::MAX)
        }
        _ => return Err(ContainerError::InvalidMvhd),
    };
    if timescale == 0 || unknown {
        return Err(ContainerError::InvalidMvhd);
    }
    let creation_unix =
        i64::try_from(creation).map_err(|_| ContainerError::InvalidMvhd)? - 2_082_844_800;
    Ok(Container {
        creation_time: UtcInstant {
            secs: creation_unix,
        },
        duration_secs: duration / u64::from(timescale),
    })
}

#[cfg(test)]
mod tests;
