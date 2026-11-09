use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use vpt_domain::digest::Sha256Digest;

pub const BUFFER: usize = 64 * 1024;

pub fn digest_open(file: &mut File) -> std::io::Result<Sha256Digest> {
    file.seek(SeekFrom::Start(0))?;
    digest_reader(file)
}

pub(crate) fn digest_reader(reader: &mut impl Read) -> std::io::Result<Sha256Digest> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; BUFFER];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Sha256Digest(hasher.finalize().into()))
}
