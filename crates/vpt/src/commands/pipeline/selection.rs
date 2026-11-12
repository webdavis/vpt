use crate::{commands::note, compose::errors::ledger_error};
use vpt_adapters::{FilesystemStores, Roots, SystemClock};
use vpt_application::{Settings, pipeline::NextStage, ports::*};
use vpt_protocol::error::ErrorDocument;

pub(super) fn next(
    ledger: &impl NoteInputsLedger,
    settings: &Settings,
    roots: &Roots,
    stores: &FilesystemStores,
    record: &RecordingRecord,
    mentions: &crate::compose::mentions::Context,
) -> Result<Option<NextStage>, Box<ErrorDocument>> {
    if let Some(stage) = vpt_application::pipeline::next(record.stages) {
        return Ok(Some(stage));
    }
    let mut inputs = ledger
        .note_inputs(&record.id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    if !inputs.has_renderable_notes() {
        return Ok(None);
    }
    mentions.enrich(&mut inputs, None)?;
    let at = SystemClock.now();
    let delta = vpt_application::note::prepare(
        &inputs,
        settings,
        vpt_application::note::WriteOptions {
            analysis_content: None,
            roots: &roots.stores,
            regenerate_expired: false,
        },
        at,
        SystemClock.offset_at(at),
        stores,
        &vpt_adapters::note_document::NoteFormatter,
    )
    .map_err(note::errors::document)?;
    let note_changed = delta.relations.is_some()
        || delta.notes.iter().any(|change| {
            change.expected_saved != Some(change.artifact.digest) || change.relocated_from.is_some()
        })
        || delta
            .commit
            .publications
            .iter()
            .any(|publication| publication.expected_previous != Some(publication.intended));
    if note_changed {
        return Ok(Some(NextStage::Note));
    }
    if verification(&inputs, settings, NoteStage::Transcript) {
        return Ok(Some(NextStage::Verify(NoteStage::Transcript)));
    }
    if analysis(&inputs, settings, roots)? {
        return Ok(Some(NextStage::Analyze));
    }
    Ok(verification(&inputs, settings, NoteStage::Analysis)
        .then_some(NextStage::Verify(NoteStage::Analysis)))
}

pub(super) fn analysis(
    input: &NoteInputs,
    settings: &Settings,
    roots: &Roots,
) -> Result<bool, Box<ErrorDocument>> {
    if settings.analysis.command.is_empty()
        || input.record.stages.note != StageState::Succeeded
        || input.record.stages.analysis == StageState::Expired
    {
        return Ok(false);
    }
    Ok(!crate::commands::analyze::resume::current(
        input, settings, roots,
    )?)
}

pub(super) fn verification(input: &NoteInputs, settings: &Settings, stage: NoteStage) -> bool {
    let state = match stage {
        NoteStage::Transcript => input.record.stages.note,
        NoteStage::Analysis => input.record.stages.analysis,
    };
    state != StageState::Expired
        && (input.review.stale_artifacts.contains(&stage)
            || input.review.verified_claims.iter().any(|claims| {
                claims.artifact == stage
                    && claims.grounding_window_secs != settings.analysis.grounding_window_secs
            }))
        && input.notes.iter().any(|note| note.stage == stage)
}
