use super::{DirtyPublication, LedgerError};
use std::{fmt, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyKind {
    Terms,
    Tags,
}

#[derive(Clone, PartialEq, Eq)]
pub struct VocabularyPublication {
    pub kind: VocabularyKind,
    pub entry: DirtyPublication,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for VocabularyPublication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VocabularyPublication")
            .field("kind", &self.kind)
            .field("entry", &self.entry)
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

pub trait VocabularyLedger {
    fn vocabulary_publication(
        &self,
        target: &Path,
    ) -> Result<Option<VocabularyPublication>, LedgerError>;
}
