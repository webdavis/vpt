pub const APPLE_SUBDIRECTORIES: [&str; 4] = [
    "Capture",
    "CaptureRecovery",
    "CloudRecordings_ckAssets",
    "EncryptedCloudRecordings",
];

use super::recorder_error;
use crate::{Kind, RootDir};
use std::path::Path;
use vpt_application::ports::{Candidate, RecorderError};
use vpt_domain::time::FileTime;

pub(super) fn named(root: &RootDir, name: &str) -> Result<Option<Candidate>, RecorderError> {
    if !name.ends_with(".m4a") {
        return Ok(None);
    }
    let path = root.path().join(name);
    let stat = root
        .stat(Path::new(name))
        .map_err(|error| recorder_error(&path, error))?;
    match stat.kind {
        Kind::File => Ok(Some(Candidate {
            path,
            file_name: name.into(),
            size: stat.size,
            mtime: FileTime {
                secs: stat.mtime_secs,
                nanos: stat.mtime_nanos,
            },
            flags: stat.flags,
        })),
        Kind::Link => Err(RecorderError::NotRegular(path)),
        Kind::Directory | Kind::Other => Ok(None),
    }
}

pub(super) fn subdirectory_counts(recordings: Option<&RootDir>) -> Vec<(String, Option<u64>)> {
    APPLE_SUBDIRECTORIES
        .iter()
        .map(|name| {
            let count = recordings
                .and_then(|recordings| recordings.open_subdirectory(name).ok())
                .and_then(|directory| directory.names().ok())
                .map(|entries| entries.len() as u64);
            ((*name).into(), count)
        })
        .collect()
}
