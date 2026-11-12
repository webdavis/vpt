use crate::cli::output::Outcome;
use crate::commands::handoff::errors;
use crate::compose::{Environment, errors::root_error, settings_from};
use std::path::Path;
use vpt_adapters::{SqliteLedger, config::planned_directory};
use vpt_application::ports::{NoteInputsLedger, NoteStage};
use vpt_domain::identity::RecordingId;
use vpt_protocol::{
    error::{ErrorDocument, ErrorKind},
    result::document,
};

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    stage: NoteStage,
) -> Outcome {
    execute(environment, config, id, stage).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    stage: NoteStage,
) -> Result<Outcome, Box<ErrorDocument>> {
    let id = RecordingId::parse(id)
        .map_err(|_| Box::new(ErrorDocument::new(ErrorKind::Usage, "invalid recording id")))?;
    let settings = settings_from(environment, config)?;
    let state = planned_directory(&settings.state_dir, "home.state_dir")
        .map_err(|error| Box::new(root_error(&error)))?;
    let ledger = SqliteLedger::open_read_only(&state)
        .map_err(|error| Box::new(crate::compose::errors::open_error(error)))?
        .ok_or_else(errors::missing)?;
    let input = ledger
        .note_inputs(&id)
        .map_err(errors::ledger)?
        .ok_or_else(errors::missing)?;
    let mut body = serde_json::json!({"recording": id.as_str(), "stage": stage.as_str()});
    match stage {
        NoteStage::Transcript => body["segments"] = super::segments::json(&input.review)?,
        NoteStage::Analysis => super::proposal::json(&input, &mut body)?,
    }
    let human = serde_json::to_string_pretty(&body)
        .map(|text| format!("{text}\n"))
        .map_err(|_| errors::document(vpt_domain::handoff::Error::Invalid))?;
    Ok(Outcome::Success {
        document: document("show", body),
        human,
        completed: Vec::new(),
        diagnostics: Vec::new(),
    })
}
