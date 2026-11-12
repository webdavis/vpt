use super::*;
use crate::{cli::args::redact::Options, commands::redact, compose::errors::ledger_error};
use vpt_domain::identity::RecordingId;

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    id: &RecordingId,
    stage: NoteStage,
    run: RunId,
    report: &mut Report,
) -> Result<(), Box<ErrorDocument>> {
    if !context.settings.share.automatic {
        return Ok(());
    }
    let input = context
        .ledger
        .note_inputs(id)
        .map_err(|e| Box::new(ledger_error(e)))?
        .ok_or_else(crate::commands::note::errors::not_found)?;
    let state = match stage {
        NoteStage::Transcript => input.record.stages.note,
        NoteStage::Analysis => input.record.stages.analysis,
    };
    if state == StageState::Expired || !input.notes.iter().any(|n| n.stage == stage) {
        return Ok(());
    }
    let options = Options {
        id: id.clone(),
        stage,
        destination: None,
        title: None,
    };
    let written = redact::write::Writer {
        ledger: context.ledger,
        settings: context.settings,
        roots: context.roots,
        options: &options,
        destination: &context.roots.stores.released,
        sink: context.sink,
        warnings: context.warnings,
    }
    .write(run, &mut false, true)?;
    if let Some(written) = written {
        report.redacted.push(written);
    }
    Ok(())
}
