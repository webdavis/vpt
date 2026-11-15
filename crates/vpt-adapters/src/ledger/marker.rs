use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{ErrorKind, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use vpt_domain::digest::Sha256Digest;
use vpt_domain::ledger_identity::WriteMark;

/// Where this machine records the last write it made to each state folder, one file per folder.
#[derive(Debug, Clone)]
pub struct Markers {
    dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerError {
    pub path: PathBuf,
    pub detail: String,
}

impl Markers {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn path_for(&self, folder: &Path) -> PathBuf {
        let digest = Sha256Digest(Sha256::digest(folder.as_os_str().as_bytes()).into());
        self.dir.join(digest.hex())
    }

    pub fn read(&self, folder: &Path) -> Result<Option<WriteMark>, MarkerError> {
        let path = self.path_for(folder);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(failure(&path, &error)),
        };
        parse(&text).map(Some).ok_or_else(|| MarkerError {
            path,
            detail: "it does not hold a ledger write".into(),
        })
    }

    pub fn write(&self, folder: &Path, mark: &WriteMark) -> Result<(), MarkerError> {
        let path = self.path_for(folder);
        let fail = |error: std::io::Error| failure(&path, &error);
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)
            .map_err(fail)?;
        let mut file = tempfile::Builder::new()
            .prefix(".marker-")
            .tempfile_in(&self.dir)
            .map_err(fail)?;
        let text = json!({"database_id": mark.database_id, "write_counter": mark.write_counter,
            "write_token": mark.write_token});
        file.write_all(text.to_string().as_bytes()).map_err(fail)?;
        file.as_file().sync_all().map_err(fail)?;
        file.persist(&path).map_err(|error| fail(error.error))?;
        Ok(())
    }

    pub fn remove(&self, folder: &Path) -> Result<(), MarkerError> {
        let path = self.path_for(folder);
        match std::fs::remove_file(&path) {
            Err(error) if error.kind() != ErrorKind::NotFound => Err(failure(&path, &error)),
            _ => Ok(()),
        }
    }
}

fn parse(text: &str) -> Option<WriteMark> {
    let value: Value = serde_json::from_str(text).ok()?;
    Some(WriteMark {
        database_id: value["database_id"].as_str()?.to_owned(),
        write_counter: value["write_counter"].as_i64()?,
        write_token: value["write_token"].as_str()?.to_owned(),
    })
}

fn failure(path: &Path, error: &std::io::Error) -> MarkerError {
    MarkerError {
        path: path.to_path_buf(),
        detail: error.kind().to_string(),
    }
}
