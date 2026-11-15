use super::checks::Context;
use crate::compose::errors::contained_error;
use crate::report::Row;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use vpt_adapters::{APPLE_SUBDIRECTORIES, ContainedError, RootDir, VoiceMemosStore};
use vpt_application::ports::RecorderStore;

const ABSENT: &str = "not created yet; Voice Memos makes it with its first recording on this Mac, \
and `vpt import` works without it";

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let recordings_dir = &context.settings.source.recordings_dir;
    let absent = recordings_dir.is_absolute()
        && recordings_dir
            .symlink_metadata()
            .is_err_and(|error| error.kind() == ErrorKind::NotFound);
    if absent {
        let skipped = "the Voice Memos folder does not exist yet";
        return std::iter::once(Row::ok("folder", ABSENT))
            .chain(APPLE_SUBDIRECTORIES.map(|name| Row::not_run(name, skipped)))
            .collect();
    }
    let source = open(recordings_dir);
    let folder = match &source {
        Ok((_, store)) => match store.candidates() {
            Ok(candidates) => Row::ok(
                "folder",
                format!("readable, {} candidates", candidates.len()),
            ),
            Err(_) => Row::issue("folder", "recordings directory could not be read"),
        },
        Err(detail) => Row::issue("folder", detail),
    };
    let subfolders = APPLE_SUBDIRECTORIES.map(|name| match &source {
        Ok((path, _)) => subfolder(path, name),
        Err(_) => Row::not_run(name, "the Voice Memos folder could not be opened"),
    });
    std::iter::once(folder).chain(subfolders).collect()
}

fn subfolder(recordings: &Path, name: &str) -> Row {
    match RootDir::open(recordings).and_then(|root| root.open_subdirectory(name)) {
        Ok(root) => match root.names() {
            Ok(names) => Row::ok(name, format!("{} entries", names.len())),
            Err(error) => Row::issue(name, contained_error(error).message),
        },
        Err(ContainedError::Io {
            kind: ErrorKind::NotFound,
            ..
        }) => Row::ok(name, "absent"),
        Err(error) => Row::issue(name, contained_error(error).message),
    }
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
        .map_err(|error| contained_error(error).message)
}
