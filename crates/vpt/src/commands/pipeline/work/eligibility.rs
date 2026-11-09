use super::*;
use crate::{
    commands::{note, synthesize},
    compose::{errors::ledger_error, synthesis_state},
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
    let current = record.stages.synthesis;
    let enabled = !context.settings.synthesis.command.is_empty();
    let next = match (current, enabled) {
        (StageState::Pending | StageState::Failed, false) => StageState::Disabled,
        (StageState::Disabled, true) => StageState::Pending,
        (StageState::Succeeded, true)
            if record.stages.note != StageState::Expired
                && !synthesize::resume::input_matches(&input, context.settings, stores)? =>
        {
            StageState::Pending
        }
        _ => current,
    };
    synthesis_state::set(
        context.ledger,
        context.sink,
        context.warnings,
        run,
        record,
        next,
    )
}
