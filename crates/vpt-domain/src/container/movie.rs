use super::{Container, ContainerError, ReadFailure};
use crate::time::UtcInstant;

pub(super) fn read(
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
        duration_ticks: duration,
        timescale,
    })
}
