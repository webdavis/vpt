use super::*;

impl Archive for ClonefileArchive {
    type Handle = ArchiveReader;

    fn stage<C>(&self, clone: C) -> Result<Staged, StageFailure>
    where
        C: FnOnce(&Path, (u64, u64), &str) -> Result<CloneKind, CloneError>,
    {
        self.stage_with(clone)
    }

    fn open(&self, path: &Path) -> Result<ArchiveReader, ArchiveError> {
        ArchiveReader::open(self.root()?, path)
    }

    fn size(&self, handle: &ArchiveReader) -> Result<u64, ArchiveError> {
        Ok(handle.initial.len())
    }

    fn read_at(
        &self,
        handle: &ArchiveReader,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<(), ReadFailure> {
        handle
            .file
            .read_exact_at(buf, offset)
            .map_err(|_| ReadFailure)
    }

    fn validate_trash_path(&self, path: &Path) -> Result<(), ArchiveError> {
        let root = self.root()?;
        root.regular(path)
            .map_err(|error| archive_error(path, error))?;
        root.revalidate()
            .map_err(|error| archive_error(&self.directory, error))
    }

    fn digest(&self, path: &Path) -> Result<Sha256Digest, ArchiveError> {
        self.digest_handle(&self.open(path)?)
    }

    fn digest_handle(&self, handle: &ArchiveReader) -> Result<Sha256Digest, ArchiveError> {
        handle.digest(self.root()?)
    }

    fn target(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    fn archived(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.ends_with(".m4a") && !name.starts_with('.'))
    }

    fn staged_leftovers(&self) -> Result<Vec<PathBuf>, ArchiveError> {
        self.list(|name| name.starts_with(STAGING_PREFIX))
    }

    fn publish(
        &self,
        staged: &Staged,
        handle: &ArchiveReader,
        target_name: &str,
    ) -> Result<Published, ArchiveError> {
        publish::exclusive(self.root()?, staged, handle, target_name)
    }

    fn sync_existing(&self, path: &Path, expected: &Sha256Digest) -> Result<(), ArchiveError> {
        publish::sync_file_and_directory(self.root()?, path, expected)
    }
}
