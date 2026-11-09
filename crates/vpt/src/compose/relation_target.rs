mod owned;
use crate::compose::errors::ledger_error;
use std::path::{Path, PathBuf};
use vpt_adapters::{SqliteLedger, note_index::Index};
use vpt_application::ports::*;
use vpt_domain::{
    identity::RecordingId,
    relations::{Kind, Target},
};
use vpt_protocol::error::ErrorDocument;
use vpt_protocol::error::ErrorKind;

pub(crate) fn resolve(
    target: &str,
    kind: Kind,
    input: &NoteInputs,
    ledger: &SqliteLedger,
    index: &Index,
) -> Result<(Target, Option<PathBuf>), Box<ErrorDocument>> {
    if let Ok(id) = RecordingId::parse(target) {
        return recording(id, &input.record.id, ledger);
    }
    let path = if Path::new(target).is_absolute() {
        index
            .notes()
            .iter()
            .find(|note| note.path == Path::new(target))
            .map(|note| &note.path)
            .ok_or_else(|| usage("target note is not indexed"))?
    } else {
        match index.matching(target) {
            [path] => path,
            [] => return Err(usage("target note not found")),
            _ => return Err(usage("target note is ambiguous")),
        }
    };
    if input.notes.iter().any(|note| note.target == *path) {
        return Err(usage("a relation cannot target its own note"));
    }
    let current = owned::read(path)?;
    if vpt_adapters::note_document::NoteFormatter
        .recording(&current.text)
        .ok()
        .flatten()
        .as_ref()
        == Some(&input.record.id)
    {
        return Err(usage("a relation cannot target its own note"));
    }
    if matches!(kind, Kind::Continues | Kind::Supersedes) {
        let id = owned::recording(path, ledger, &current.text)?;
        return recording(id, &input.record.id, ledger);
    }
    let target = path
        .to_str()
        .ok_or_else(|| usage("target note path must be UTF-8"))?;
    Ok((Target::Note(target.into()), Some(path.clone())))
}

fn recording(
    id: RecordingId,
    source: &RecordingId,
    ledger: &SqliteLedger,
) -> Result<(Target, Option<PathBuf>), Box<ErrorDocument>> {
    if &id == source {
        return Err(usage("a relation cannot target its own recording"));
    }
    ledger
        .by_id(&id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(|| usage("target recording not found"))?;
    let path = ledger
        .note_path(&id, NoteStage::Transcript)
        .map_err(|error| Box::new(ledger_error(error)))?;
    Ok((Target::Recording(id), path))
}

fn usage(message: &str) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(ErrorKind::Usage, message))
}
