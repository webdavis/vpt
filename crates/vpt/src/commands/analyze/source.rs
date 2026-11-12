use super::errors;
use vpt_adapters::{FilesystemStores, note_document::Document};
use vpt_application::ports::{NoteArtifact, NoteFiles, Stores};
use vpt_protocol::error::ErrorDocument;

pub(super) fn content(source: &NoteArtifact) -> Result<String, Box<ErrorDocument>> {
    Ok(Document::parse(&source.text)
        .map_err(|_| errors::invalid())?
        .content()
        .map_err(|_| errors::invalid())?
        .to_owned())
}

pub(super) fn check(
    stores: &FilesystemStores,
    source: &NoteArtifact,
) -> Result<(), Box<ErrorDocument>> {
    let disk = stores
        .read_note(&source.target)
        .map_err(|_| errors::changed())?
        .ok_or_else(errors::changed)?;
    let document = Document::parse(&disk.text).map_err(|_| errors::changed())?;
    document
        .check_owner(&source.recording, "transcript")
        .map_err(|_| errors::changed())?;
    if stores.digest_bytes(
        document
            .content()
            .map_err(|_| errors::changed())?
            .as_bytes(),
    ) != source.content_digest
    {
        return Err(errors::changed());
    }
    Ok(())
}
