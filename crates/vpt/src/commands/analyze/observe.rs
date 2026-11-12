use super::errors;
use crate::{
    cli::output::Outcome,
    commands::note,
    compose::errors::{contained_error, ledger_error},
};
use vpt_adapters::{FilesystemStores, Roots, note_document::NoteFormatter};
use vpt_application::{Settings, ports::*};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn inputs(
    ledger: &impl NoteInputsLedger,
    id: &RecordingId,
) -> Result<NoteInputs, Box<ErrorDocument>> {
    let input = ledger
        .note_inputs(id)
        .map_err(|error| Box::new(ledger_error(error)))?
        .ok_or_else(note::errors::not_found)?;
    if input.review.accepted.is_none() {
        return Err(errors::usage("recording has no accepted transcript"));
    }
    if input.record.stages.note == StageState::Expired
        || input.record.stages.analysis == StageState::Expired
    {
        return Err(Box::new(
            ErrorDocument::new(
                ErrorKind::Refused,
                "note stage has expired; use note write to regenerate it",
            )
            .rule("expired_note"),
        ));
    }
    Ok(input)
}

pub(super) fn plan(
    input: NoteInputs,
    settings: &Settings,
    roots: &Roots,
) -> Result<Outcome, Box<ErrorDocument>> {
    let content = content(&input, settings)?;
    let source = match input
        .notes
        .iter()
        .find(|note| note.stage == NoteStage::Transcript)
    {
        Some(note) => note.target.clone(),
        None => roots.stores.transcripts.join(
            vpt_domain::note::filename::transcript_name(
                &input.record.id,
                input.record.title.as_deref(),
                settings.note.slug_max_chars as usize,
            )
            .map_err(|_| errors::invalid())?,
        ),
    };
    let stores =
        FilesystemStores::open_read_only(&[]).map_err(|error| Box::new(contained_error(error)))?;
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: vpt_protocol::result::document(
            "analyze",
            serde_json::json!({
                "id": input.record.id.as_str(), "dry_run": true, "transcript": source,
                "input_digest": stores.digest_bytes(content.as_bytes()).hex(), "timeout_secs": settings.analysis.timeout.as_secs(),
                "operations": ["write transcript", "run analysis command", "verify analysis", "publish notes"]
            }),
        ),
        human: format!(
            "Would analyze {} and publish its analysis note\n",
            input.record.id
        ),
    })
}

pub(super) fn content(
    input: &NoteInputs,
    settings: &Settings,
) -> Result<String, Box<ErrorDocument>> {
    let accepted = input.review.accepted.as_ref().ok_or_else(errors::invalid)?;
    let content = vpt_domain::note::transcript_content(
        &accepted.transcript,
        &input.review.flags,
        settings.readability_enabled,
    )
    .map_err(|_| errors::invalid())?;
    let content = NoteFormatter
        .render_findings(&content, NoteStage::Transcript, &input.review.flags)
        .map_err(|_| errors::invalid())?;
    Ok(content)
}
