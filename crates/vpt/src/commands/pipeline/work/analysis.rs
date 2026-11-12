use super::*;
use crate::{
    commands::{analyze, note},
    compose::errors::ledger_error,
};
use vpt_domain::identity::RecordingId;
use vpt_domain::run::RunKind;

pub(super) fn run<E: EventSink>(
    context: &mut Context<'_, E>,
    mentions: &crate::compose::mentions::Context,
    id: &RecordingId,
    report: &mut Report,
) -> Result<(), Box<ErrorDocument>> {
    let input = context
        .ledger
        .note_inputs(id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    if !super::super::selection::analysis(&input, context.settings, context.roots)? {
        return Ok(());
    }
    let run = commit(
        context
            .ledger
            .begin_run(RunKind::Analyze, SystemClock.now()),
        context.sink,
        context.warnings,
    )?
    .run;
    let mut committed = false;
    let result = analyze::work::Writer {
        mentions: Some(mentions),
        settings: context.settings,
        roots: context.roots,
        ledger: context.ledger,
        sink: context.sink,
        warnings: context.warnings,
    }
    .run(input, run, &mut committed)
    .map(|outcome| report.record_diagnostics(id, outcome));
    super::child::finish(context, run, id, committed, false, result)?;
    report.analyzed.push(id.clone());
    Ok(())
}
