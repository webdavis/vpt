use super::*;
use crate::{
    commands::{note, transcribe},
    compose::errors::ledger_error,
};
use vpt_application::pipeline::NextStage;
use vpt_domain::run::RunKind;

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    stores: &FilesystemStores,
    mentions: &crate::compose::mentions::Context,
    record: &RecordingRecord,
    run: RunId,
    report: &mut Report,
    notices: &mut ReviewQueue,
) -> Result<(), Box<ErrorDocument>> {
    match super::super::selection::next(
        context.ledger,
        context.settings,
        context.roots,
        stores,
        record,
        mentions,
    )? {
        None | Some(NextStage::Analyze | NextStage::Verify(_) | NextStage::Redact(_)) => {}
        Some(NextStage::Transcribe) => {
            let duration = transcribe::observe::audio(record, context.settings, context.roots)?;
            let child = commit(
                context
                    .ledger
                    .begin_run(RunKind::Transcribe, SystemClock.now()),
                context.sink,
                context.warnings,
            )?
            .run;
            let outcome = transcribe::work::run(
                transcribe::work::Context {
                    mentions: Some(mentions),
                    ledger: context.ledger,
                    stores,
                    settings: context.settings,
                    roots: context.roots,
                    sink: context.sink,
                    warnings: context.warnings,
                    notices: Some(notices),
                },
                record.clone(),
                duration,
                child,
            )?;
            report.record_diagnostics(&record.id, outcome);
            report.transcribed.push(record.id.clone());
            if record.stages.note != StageState::Expired {
                let path = context
                    .ledger
                    .note_path(&record.id, NoteStage::Transcript)
                    .map_err(|error| Box::new(ledger_error(error)))?
                    .ok_or_else(note::errors::not_found)?;
                report.notes.push((record.id.clone(), path));
            }
        }
        Some(NextStage::Note) => {
            let inputs = context
                .ledger
                .note_inputs(&record.id)
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
            .write(inputs, run, &mut false)?;
            report.notes.push((record.id.clone(), path.clone()));
            if let Some(notice) =
                crate::compose::review_notice::read(context.ledger, &record.id, path)?
            {
                notices.push(notice).map_err(|_| {
                    Box::new(ErrorDocument::new(
                        ErrorKind::Ledger,
                        "review alert capacity exceeded",
                    ))
                })?;
            }
        }
    }
    Ok(())
}
