use super::{prepare, source::Source, unchanged};
use crate::cli::args::redact::Options;
use vpt_adapters::{ReleaseFiles, Roots};
use vpt_application::{Settings, ports::*};
use vpt_protocol::error::ErrorDocument;

pub(crate) fn next(
    ledger: &(impl NoteInputsLedger + ReleaseLedger),
    settings: &Settings,
    roots: &Roots,
    record: &RecordingRecord,
) -> Result<Option<NoteStage>, Box<ErrorDocument>> {
    if !settings.share.automatic {
        return Ok(None);
    }
    let input = ledger
        .note_inputs(&record.id)
        .map_err(super::errors::ledger)?
        .ok_or_else(super::errors::missing)?;
    let destination = ReleaseFiles::planned_destination(roots, &roots.stores.released)
        .map_err(crate::compose::releases::errors::document)?;
    for (stage, state) in [
        (NoteStage::Transcript, record.stages.note),
        (NoteStage::Analysis, record.stages.analysis),
    ] {
        if state == StageState::Expired || !input.notes.iter().any(|n| n.stage == stage) {
            continue;
        }
        let options = Options {
            id: record.id.clone(),
            stage,
            destination: None,
            title: None,
        };
        let source = Source::read(ledger, roots, &record.id, stage)?;
        let prepared = prepare::run(&source, &options, settings, roots)?;
        let digest = source.stores.digest_bytes(prepared.content.text.as_bytes());
        if !unchanged::matches(ledger, &record.id, stage, &destination, digest)? {
            return Ok(Some(stage));
        }
    }
    Ok(None)
}
