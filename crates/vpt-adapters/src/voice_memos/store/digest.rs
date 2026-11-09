use sha2::{Digest, Sha256};
use std::fs::File;
use std::os::unix::fs::FileExt;
use vpt_application::ports::RecorderError;
use vpt_domain::digest::Sha256Digest;

pub(super) fn bounded(file: &File, size: u64) -> Result<Sha256Digest, RecorderError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut offset = 0;
    while offset < size {
        let count =
            usize::try_from((size - offset).min(buffer.len() as u64)).unwrap_or(buffer.len());
        file.read_exact_at(&mut buffer[..count], offset)
            .map_err(|error| RecorderError::Io(error.kind().to_string()))?;
        hasher.update(&buffer[..count]);
        offset += count as u64;
    }
    Ok(Sha256Digest(hasher.finalize().into()))
}
