use super::usage;
use crate::compose::errors::{contained_error, ledger_error, store_error};
use std::path::Path;
use vpt_adapters::{FilesystemStores, SqliteLedger, note_document::Document};
use vpt_application::ports::{NoteFiles, NoteLedger, StoredNote};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

pub(super) fn recording(
    path: &Path,
    ledger: &SqliteLedger,
    current: &str,
) -> Result<RecordingId, Box<ErrorDocument>> {
    let saved = ledger
        .note_at(path)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(|| usage("this relation kind requires a recording or owned note target"))?;
    Document::parse(current)
        .and_then(|document| document.check_owner(&saved.recording, saved.stage.as_str()))
        .map_err(|_| usage("target note does not match saved ownership"))?;
    Ok(saved.recording)
}

pub(super) fn read(path: &Path) -> Result<StoredNote, Box<ErrorDocument>> {
    let parent = path
        .parent()
        .ok_or_else(|| usage("target note path is invalid"))?;
    let stores = FilesystemStores::open_read_only(&[parent.to_path_buf()])
        .map_err(|error| Box::new(contained_error(error)))?;
    stores
        .read_note(path)
        .map_err(|error| Box::new(store_error(error)))?
        .ok_or_else(|| usage("target note is missing"))
}
