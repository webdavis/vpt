mod boundaries;
mod reader;
mod staging;

use super::*;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use vpt_application::ports::RecorderStore;

fn fixture(bytes: &[u8]) -> (tempfile::TempDir, PathBuf, crate::VoiceMemosStore, File) {
    let temp = crate::test_temp::tempdir().expect("temp");
    let base = temp.path().canonicalize().expect("canonical");
    let recordings = base.join("Recordings");
    let audio = base.join("audio");
    std::fs::create_dir(&recordings).expect("recordings");
    std::fs::create_dir(&audio).expect("audio");
    let source = recordings.join("source.m4a");
    std::fs::write(&source, bytes).expect("source");
    let store = crate::VoiceMemosStore::open(&recordings).expect("store");
    let handle = store.open(&source).expect("handle");
    (temp, audio, store, handle)
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777
}
