use super::errors;
use std::path::Path;
use vpt_adapters::{
    FilesystemStores,
    note_document::{Document, NoteFormatter},
};
use vpt_application::ports::{NoteFiles, NoteFormat, NoteInputs, NoteStage, StoredNote};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Source {
    pub note: StoredNote,
    pub stores: FilesystemStores,
    pub stage: Option<NoteStage>,
}

pub(super) fn read(
    path: &Path,
    input: &NoteInputs,
    roots: &vpt_adapters::Roots,
) -> Result<Source, Box<ErrorDocument>> {
    let canonical = path.canonicalize().map_err(|_| errors::read())?;
    let parent = canonical.parent().ok_or_else(errors::read)?;
    let stores = FilesystemStores::open_read_only(&[parent.into()]).map_err(|_| errors::read())?;
    let note = stores
        .read_note(&canonical)
        .map_err(|_| errors::read())?
        .ok_or_else(errors::read)?;
    if let Ok(Some(recording)) = NoteFormatter.recording(&note.text)
        && recording != input.record.id
    {
        return Err(errors::ownership());
    }
    let stage = if path == canonical {
        input
            .notes
            .iter()
            .find(|note| note.target == path)
            .and_then(|owned| {
                let root = match owned.stage {
                    NoteStage::Transcript => &roots.stores.transcripts,
                    NoteStage::Analysis => &roots.stores.analysis,
                };
                if !path.starts_with(root) || path == root {
                    return None;
                }
                let parsed = Document::parse(&note.text).ok()?;
                parsed
                    .check_owner(&input.record.id, owned.stage.as_str())
                    .ok()?;
                Some(owned.stage)
            })
    } else {
        None
    };
    Ok(Source {
        note,
        stores,
        stage,
    })
}
