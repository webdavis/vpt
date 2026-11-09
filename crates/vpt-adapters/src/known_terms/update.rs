use super::{TermsError, document, parent::Parent, snapshot};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use vpt_application::ports::{DirtyPublication, VocabularyPublication};
use vpt_domain::{digest::Sha256Digest, time::UtcInstant};
mod write;

pub struct Update {
    parent: Parent,
    path: PathBuf,
    previous: Option<Sha256Digest>,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for Update {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VocabularyUpdate")
    }
}

pub fn prepare(path: &Path, term: &str) -> Result<Update, TermsError> {
    if !vpt_domain::review::valid_correction(term) {
        return Err(TermsError::InvalidText);
    }
    let term = term.split_whitespace().collect::<Vec<_>>().join(" ");
    prepare_with(path, |bytes| document::append(bytes, &term))
}

pub(crate) fn prepare_with(
    path: &Path,
    render: impl FnOnce(&[u8]) -> Result<Vec<u8>, TermsError>,
) -> Result<Update, TermsError> {
    let parent = Parent::capture(path)?;
    let previous = parent
        .existing()
        .map(|root| snapshot::read(root, path))
        .transpose()?
        .flatten();
    if previous.as_ref().is_some_and(|old| !old.single_link) {
        return Err(TermsError::UnsafePath);
    }
    let previous = previous.map(|old| old.bytes);
    let bytes = render(previous.as_deref().unwrap_or_default())?;
    Ok(Update {
        parent,
        path: path.into(),
        previous: previous.as_deref().map(digest),
        bytes,
    })
}

impl Update {
    pub fn restore(publication: &DirtyPublication, bytes: Vec<u8>) -> Result<Self, TermsError> {
        let saved = VocabularyPublication {
            kind: vpt_application::ports::VocabularyKind::Terms,
            entry: publication.clone(),
            bytes,
        };
        Self::restore_saved(saved)
    }

    pub fn restore_saved(saved: VocabularyPublication) -> Result<Self, TermsError> {
        super::validation::validate(&saved)?;
        let publication = &saved.entry;
        Ok(Self {
            parent: Parent::capture(&publication.target)?,
            path: publication.target.clone(),
            previous: publication.expected_previous,
            bytes: saved.bytes,
        })
    }

    pub fn publication(&self, at: UtcInstant) -> DirtyPublication {
        DirtyPublication {
            target: self.path.clone(),
            expected_previous: self.previous,
            intended: digest(&self.bytes),
            recorded_at: at,
        }
    }

    pub fn content(&self) -> &[u8] {
        &self.bytes
    }
    pub fn added(&self) -> bool {
        self.previous != Some(digest(&self.bytes))
    }
    /// The caller holds VPT's write lock and journals the publication and content before writing.
    pub fn publish(&self) -> Result<(), TermsError> {
        let root = self.parent.create()?;
        write::publish(&root, &self.path, self.previous, &self.bytes)
    }
}

fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}

#[cfg(test)]
mod tests;
