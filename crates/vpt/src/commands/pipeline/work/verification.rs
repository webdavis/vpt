use super::*;
use crate::{
    commands::{note, verify},
    compose::errors::ledger_error,
};
use vpt_domain::{identity::RecordingId, run::RunKind};

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    stores: &FilesystemStores,
    mentions: &crate::compose::mentions::Context,
    id: &RecordingId,
    stage: NoteStage,
    report: &mut Report,
) -> Result<(), Box<ErrorDocument>> {
    let input = context
        .ledger
        .note_inputs(id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    if !super::super::selection::verification(&input, context.settings, stage) {
        return Ok(());
    }
    let path = input
        .notes
        .iter()
        .find(|note| note.stage == stage)
        .ok_or_else(note::errors::not_found)?
        .target
        .clone();
    let run = commit(
        context
            .ledger
            .begin_run(RunKind::VerifyNote, SystemClock.now()),
        context.sink,
        context.warnings,
    )?
    .run;
    let mut committed = false;
    let result = (|| {
        verify::persist::Writer {
            settings: context.settings,
            roots: context.roots,
            ledger: context.ledger,
            sink: context.sink,
            warnings: context.warnings,
        }
        .run(&path, input, run, &mut committed)?;
        let input = context
            .ledger
            .note_inputs(id)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(note::errors::not_found)?;
        let path = note::write::Writer {
            mentions,
            ledger: context.ledger,
            stores,
            settings: context.settings,
            options: vpt_application::note::WriteOptions {
                analysis_content: None,
                roots: &context.roots.stores,
                regenerate_expired: false,
            },
            sink: context.sink,
            warnings: context.warnings,
        }
        .write(input, run, &mut committed)?;
        if let Some((_, previous_path)) = report
            .notes
            .last_mut()
            .filter(|(previous, _)| previous == id)
        {
            *previous_path = path;
        } else {
            report.notes.push((id.clone(), path));
        }
        Ok(())
    })();
    super::child::finish(context, run, id, committed, true, result)
}
