use super::{FilesystemStores, notes::read::ReadError};
use crate::note_document;
use vpt_application::ports::{
    ArtifactKind, ArtifactOwner, RetentionArtifact, RetentionCheck, RetentionFiles, StoreError,
    Stores,
};

impl RetentionFiles for FilesystemStores {
    fn check_retention(&self, artifact: &RetentionArtifact) -> Result<RetentionCheck, StoreError> {
        if !artifact.owner.matches_kind(artifact.kind) {
            return Ok(RetentionCheck::Modified);
        }
        if vpt_domain::brief::document::source_path(&artifact.path).is_none() {
            return Err(StoreError::Escape(artifact.path.clone()));
        }
        self.validate_path(&artifact.path)?;
        if matches!(
            artifact.kind,
            ArtifactKind::Transcript | ArtifactKind::Analysis | ArtifactKind::Brief
        ) {
            return self.check_retention_note(artifact);
        }
        Ok(match self.digest_of(&artifact.path)? {
            None => RetentionCheck::Missing,
            Some(digest) if digest == artifact.digest => RetentionCheck::Owned(digest),
            Some(_) => RetentionCheck::Modified,
        })
    }
}

impl FilesystemStores {
    fn check_retention_note(
        &self,
        artifact: &RetentionArtifact,
    ) -> Result<RetentionCheck, StoreError> {
        let bytes =
            match self.read_note_bytes(&artifact.path, vpt_domain::note::MAX_RENDERED, None, || {})
            {
                Ok(Some(bytes)) => bytes,
                Ok(None) => return Ok(RetentionCheck::Missing),
                Err(ReadError::TooLarge) => return Ok(RetentionCheck::Modified),
                Err(ReadError::Store(error)) => return Err(error),
            };
        let Ok(text) = std::str::from_utf8(&bytes.bytes) else {
            return Ok(RetentionCheck::Modified);
        };
        let owned = match &artifact.owner {
            ArtifactOwner::Recording(id) => note_document::Document::parse(text)
                .and_then(|document| document.check_owner(id, artifact.kind.as_str())),
            ArtifactOwner::Occasion(id) => note_document::brief::Document::parse(text)
                .and_then(|document| document.check_owner(id)),
        };
        Ok(if owned.is_ok() {
            RetentionCheck::Owned(bytes.digest)
        } else {
            RetentionCheck::Modified
        })
    }
}

#[cfg(test)]
mod tests;
