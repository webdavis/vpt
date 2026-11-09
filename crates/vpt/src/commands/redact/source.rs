use super::errors;
use vpt_adapters::{ContainedError, FilesystemStores, Roots, redaction::document::owned_content};
use vpt_application::ports::*;
use vpt_domain::{identity::RecordingId, note::NoteStage};
use vpt_protocol::error::ErrorDocument;
pub(super) struct Source {
    pub input: NoteInputs,
    pub path: std::path::PathBuf,
    pub note: StoredNote,
    pub offset: usize,
    pub stores: FilesystemStores,
}
impl Source {
    pub fn read(
        ledger: &impl NoteInputsLedger,
        roots: &Roots,
        id: &RecordingId,
        stage: NoteStage,
    ) -> Result<Self, Box<ErrorDocument>> {
        let input = ledger
            .note_inputs(id)
            .map_err(errors::ledger)?
            .ok_or_else(errors::missing)?;
        let artifact = input
            .notes
            .iter()
            .find(|n| n.stage == stage)
            .ok_or_else(errors::missing)?;
        let root = match stage {
            NoteStage::Transcript => &roots.stores.transcripts,
            NoteStage::Analysis => &roots.stores.analysis,
        };
        if artifact.target.parent() != Some(root.as_path()) {
            return Err(errors::source());
        }
        let stores =
            FilesystemStores::open(std::slice::from_ref(root)).map_err(|error| match error {
                ContainedError::Io {
                    kind: std::io::ErrorKind::NotFound,
                    ..
                } => errors::missing(),
                other => Box::new(crate::compose::errors::contained_error(other)),
            })?;
        let note = stores
            .read_note(&artifact.target)
            .map_err(|e| Box::new(crate::compose::errors::store_error(e)))?
            .ok_or_else(errors::missing)?;
        let (content, offset) = owned_content(&note.text, id, stage).map_err(errors::redaction)?;
        if stores.digest_bytes(content.as_bytes()) != artifact.content_digest {
            return Err(errors::source());
        }
        let path = artifact.target.clone();
        Ok(Self {
            input,
            path,
            note,
            offset,
            stores,
        })
    }
    pub fn unchanged(&self) -> Result<(), Box<ErrorDocument>> {
        if self
            .stores
            .digest_of(&self.path)
            .map_err(|e| Box::new(crate::compose::errors::store_error(e)))?
            != Some(self.note.digest)
        {
            return Err(errors::source());
        }
        Ok(())
    }
}
