use crate::container::{Container, ContainerError, ReadFailure, inspect};

const MAC_EPOCH_OFFSET: i64 = 2_082_844_800;

pub fn reader(bytes: &[u8]) -> impl FnMut(u64, &mut [u8]) -> Result<(), ReadFailure> + '_ {
    move |offset, buf| {
        let start = usize::try_from(offset).map_err(|_| ReadFailure)?;
        let end = start.checked_add(buf.len()).ok_or(ReadFailure)?;
        buf.copy_from_slice(bytes.get(start..end).ok_or(ReadFailure)?);
        Ok(())
    }
}

pub fn inspect_bytes(bytes: &[u8]) -> Result<Container, ContainerError> {
    inspect(bytes.len() as u64, reader(bytes))
}

pub fn box_of(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = ((8 + body.len()) as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out
}

pub fn mvhd(creation_unix: i64, duration_secs: u32) -> Vec<u8> {
    let mac = (creation_unix + MAC_EPOCH_OFFSET) as u32;
    let mut body = vec![0u8; 4];
    body.extend(mac.to_be_bytes());
    body.extend(mac.to_be_bytes());
    body.extend(1_000u32.to_be_bytes());
    body.extend((duration_secs * 1_000).to_be_bytes());
    body.extend([0u8; 80]);
    box_of(b"mvhd", &body)
}

pub fn mvhd_v1(creation_unix: i64, duration_secs: u64) -> Vec<u8> {
    let mac = (creation_unix + MAC_EPOCH_OFFSET) as u64;
    let mut body = vec![1u8, 0, 0, 0];
    body.extend(mac.to_be_bytes());
    body.extend(mac.to_be_bytes());
    body.extend(1_000u32.to_be_bytes());
    body.extend((duration_secs * 1_000).to_be_bytes());
    body.extend([0u8; 80]);
    box_of(b"mvhd", &body)
}

pub fn m4a(creation_unix: i64, duration_secs: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = box_of(b"ftyp", b"M4A \0\0\0\0M4A mp42isom");
    out.extend(box_of(b"mdat", payload));
    out.extend(box_of(b"moov", &mvhd(creation_unix, duration_secs)));
    out
}
