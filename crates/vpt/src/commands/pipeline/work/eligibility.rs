use super::*;
use crate::{
    commands::{analyze, note},
    compose::{analysis_state, errors::ledger_error},
};

pub(super) fn refresh<E: EventSink>(
    context: &mut Context<'_, E>,
    stores: &FilesystemStores,
    run: RunId,
    record: &mut RecordingRecord,
) -> Result<(), Box<ErrorDocument>> {
    let input = context
        .ledger
        .note_inputs(&record.id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    *record = input.record.clone();
    let current = record.stages.analysis;
    let enabled = !context.settings.analysis.command.is_empty();
    let next = match (current, enabled) {
        (StageState::Pending | StageState::Failed, false) => StageState::Disabled,
        (StageState::Disabled, true) => StageState::Pending,
        (StageState::Succeeded, true)
            if record.stages.note != StageState::Expired
                && !analyze::resume::input_matches(&input, context.settings, stores)? =>
        {
            StageState::Pending
        }
        _ => current,
    };
    analysis_state::set(
        context.ledger,
        context.sink,
        context.warnings,
        run,
        record,
        next,
    )
}
