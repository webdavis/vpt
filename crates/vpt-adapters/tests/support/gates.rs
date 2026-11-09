use std::cell::Cell;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::Path;
use vpt_adapters::VoiceMemosStore;
use vpt_application::ports::{
    Candidate, CloneError, CloneKind, RecorderError, RecorderStore, SourceMetadata, TitleLookup,
};
use vpt_domain::container::ReadFailure;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloneBehavior {
    Real,
    ByteCopy,
    Truncated,
}

pub struct GateRecorder {
    pub inner: VoiceMemosStore,
    pub extra_flags: Cell<u32>,
    pub opens: Cell<u32>,
    pub fail_open_after: Cell<Option<u32>>,
    pub reads: Cell<u32>,
    pub clones: Cell<u32>,
    pub metadata_calls: Cell<u32>,
    pub grow_after_stage: Cell<bool>,
    pub clone_behavior: Cell<CloneBehavior>,
}

impl GateRecorder {
    pub fn new(recordings: &Path) -> Self {
        Self {
            inner: VoiceMemosStore::open(recordings).unwrap(),
            extra_flags: Cell::new(0),
            opens: Cell::new(0),
            fail_open_after: Cell::new(None),
            reads: Cell::new(0),
            clones: Cell::new(0),
            metadata_calls: Cell::new(0),
            grow_after_stage: Cell::new(false),
            clone_behavior: Cell::new(CloneBehavior::Real),
        }
    }
}

impl RecorderStore for GateRecorder {
    type Handle = File;

    fn digest(
        &self,
        handle: &File,
        size: u64,
    ) -> Result<vpt_domain::digest::Sha256Digest, RecorderError> {
        self.reads.set(self.reads.get() + 1);
        self.inner.digest(handle, size)
    }

    fn candidates(&self) -> Result<Vec<Candidate>, RecorderError> {
        self.inner.candidates().map(|mut candidates| {
            for candidate in &mut candidates {
                candidate.flags |= self.extra_flags.get();
            }
            candidates
        })
    }

    fn candidate(&self, path: &Path) -> Result<Candidate, RecorderError> {
        self.inner.candidate(path).map(|mut candidate| {
            candidate.flags |= self.extra_flags.get();
            candidate
        })
    }

    fn open(&self, path: &Path) -> Result<File, RecorderError> {
        self.opens.set(self.opens.get() + 1);
        if self
            .fail_open_after
            .get()
            .is_some_and(|limit| self.opens.get() > limit)
        {
            return Err(RecorderError::Unreadable("private source failure".into()));
        }
        self.inner.open(path)
    }

    fn metadata(&self, handle: &File) -> Result<SourceMetadata, RecorderError> {
        let mut metadata = self.inner.metadata(handle)?;
        let calls = self.metadata_calls.get() + 1;
        self.metadata_calls.set(calls);
        if self.grow_after_stage.get() && calls >= 2 {
            metadata.size += 1;
        }
        Ok(metadata)
    }

    fn read_at(&self, handle: &File, offset: u64, buf: &mut [u8]) -> Result<(), ReadFailure> {
        self.reads.set(self.reads.get() + 1);
        self.inner.read_at(handle, offset, buf)
    }

    fn clone_into(
        &self,
        handle: &File,
        directory: &Path,
        identity: (u64, u64),
        name: &str,
    ) -> Result<CloneKind, CloneError> {
        self.clones.set(self.clones.get() + 1);
        match self.clone_behavior.get() {
            CloneBehavior::Real => self.inner.clone_into(handle, directory, identity, name),
            CloneBehavior::ByteCopy => {
                copy_bytes(handle, directory, name, 0).map(|()| CloneKind::ByteCopy)
            }
            CloneBehavior::Truncated => {
                copy_bytes(handle, directory, name, 12).map(|()| CloneKind::CopyOnWrite)
            }
        }
    }

    fn subdirectory_counts(&self) -> Vec<(String, Option<u64>)> {
        self.inner.subdirectory_counts()
    }

    fn title(&self, file_name: &str) -> TitleLookup {
        self.inner.title(file_name)
    }

    fn refresh_titles(&self) -> Result<(), RecorderError> {
        self.inner.refresh_titles()
    }
}

fn copy_bytes(source: &File, directory: &Path, name: &str, tail: usize) -> Result<(), CloneError> {
    let length = usize::try_from(
        source
            .metadata()
            .map_err(|error| CloneError::Io(error.to_string()))?
            .len(),
    )
    .map_err(|_| CloneError::Io("owned fixture too large".into()))?;
    let mut bytes = vec![0u8; length];
    source
        .read_exact_at(&mut bytes, 0)
        .map_err(|error| CloneError::Io(error.to_string()))?;
    bytes.truncate(length.saturating_sub(tail));
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join(name))
        .map_err(|error| CloneError::Io(error.to_string()))?;
    std::io::Write::write_all(&mut target, &bytes)
        .map_err(|error| CloneError::Io(error.to_string()))
}
