use super::{
    artifact::{self, Target},
    errors,
};
use crate::cli::output::Outcome;
use crate::compose::Environment;
use std::path::Path;
use vpt_application::ports::NoteInputsLedger;
use vpt_domain::{copy::Kind, identity::RecordingId, occasion::OccasionId};
use vpt_protocol::{
    error::{ErrorDocument, ErrorKind},
    result::document,
};

pub(crate) const BRIEF_REDACT: &str =
    "--redact does not apply to --stage brief, which is not a redaction source";

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    stage: Kind,
    redact: bool,
) -> Outcome {
    execute(environment, config, id, stage, redact).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    stage: Kind,
    redact: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let target = target(id, stage)?;
    let (settings, roots, ledger) = artifact::open(environment, config)?;
    let mut body = match (&target, redact) {
        (Target::Recording(id), true) => {
            super::redacted::copy(&settings, &roots, &ledger, id, stage)?
        }
        (Target::Occasion(_), true) => {
            return Err(Box::new(ErrorDocument::new(ErrorKind::Usage, BRIEF_REDACT)));
        }
        (_, false) => artifact::read(&ledger, &roots, &target, stage)?,
    };
    body["stage"] = stage.as_str().into();
    if let Target::Recording(id) = &target
        && !redact
        && matches!(stage, Kind::Transcript | Kind::Analysis)
    {
        let input = ledger
            .note_inputs(id)
            .map_err(errors::ledger)?
            .ok_or_else(errors::missing)?;
        match stage {
            Kind::Transcript => body["segments"] = super::segments::json(&input.review)?,
            _ => super::proposal::json(&input, &mut body)?,
        }
    }
    let human = body["content"]
        .as_str()
        .ok_or_else(errors::changed)?
        .to_owned();
    Ok(Outcome::Success {
        document: document("show", body),
        human,
        completed: Vec::new(),
        diagnostics: Vec::new(),
    })
}

fn target(id: &str, stage: Kind) -> Result<Target, Box<ErrorDocument>> {
    let usage = |message: &str| Box::new(ErrorDocument::new(ErrorKind::Usage, message));
    match stage {
        Kind::Brief => OccasionId::parse(id)
            .map(Target::Occasion)
            .map_err(|_| usage("invalid occasion id")),
        _ => RecordingId::parse(id)
            .map(Target::Recording)
            .map_err(|_| usage("invalid recording id")),
    }
}
