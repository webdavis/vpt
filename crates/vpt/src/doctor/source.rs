use super::checks::{check, notice};
use super::probes;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use vpt_adapters::{APPLE_SUBDIRECTORIES, VoiceMemosStore};
use vpt_application::ports::RecorderStore;
use vpt_protocol::error::Check;

const ABSENT: &str = "absent: Voice Memos has not stored a recording on this Mac yet; the folder \
appears after the first recording, and `vpt import` works without it";

pub(super) fn inspect(recordings_dir: &Path) -> (Check, Vec<Check>) {
    let absent = recordings_dir.is_absolute()
        && recordings_dir
            .symlink_metadata()
            .is_err_and(|error| error.kind() == ErrorKind::NotFound);
    if absent {
        return (
            notice("recordings_dir", ABSENT),
            APPLE_SUBDIRECTORIES
                .iter()
                .map(|name| {
                    notice(
                        format!("subdirectory:{name}"),
                        "not run: recordings_dir does not exist yet",
                    )
                })
                .collect(),
        );
    }
    let source = open(recordings_dir);
    let recordings = match &source {
        Ok((_, store)) => match store.candidates() {
            Ok(candidates) => check(
                "recordings_dir",
                true,
                format!("readable, {} candidates", candidates.len()),
            ),
            Err(_) => check(
                "recordings_dir",
                false,
                "recordings directory could not be read",
            ),
        },
        Err(detail) => check("recordings_dir", false, detail),
    };
    let subdirectories = APPLE_SUBDIRECTORIES
        .iter()
        .map(|name| match &source {
            Ok((path, _)) => probes::subdirectory(path, name),
            Err(_) => check(
                format!("subdirectory:{name}"),
                false,
                "not run: recordings_dir could not be opened",
            ),
        })
        .collect();
    (recordings, subdirectories)
}

fn open(recordings_dir: &Path) -> Result<(PathBuf, VoiceMemosStore), String> {
    if !recordings_dir.is_absolute() {
        return Err("source.recordings_dir must be absolute".into());
    }
    let path = recordings_dir
        .canonicalize()
        .map_err(|error| error.kind().to_string())?;
    VoiceMemosStore::open(&path)
        .map(|store| (path, store))
        .map_err(|error| crate::compose::errors::contained_error(error).message)
}
