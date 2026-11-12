use crate::cli::{args::handoff::Target, output::Outcome};
use crate::commands::handoff::{self, errors};
use crate::compose::Environment;
use serde_json::{Value, json};
use std::path::Path;
use vpt_application::ports::NoteInputsLedger;
use vpt_domain::{handoff::Kind, identity::RecordingId, occasion::OccasionId};
use vpt_protocol::{
    error::{ErrorDocument, ErrorKind},
    result::document,
};

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    stage: Kind,
) -> Outcome {
    execute(environment, config, id, stage).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    stage: Kind,
) -> Result<Outcome, Box<ErrorDocument>> {
    let target = target(id, stage)?;
    let (_, roots, ledger) = handoff::open(environment, config)?;
    let mut body = json!({"stage": stage.as_str()});
    flatten(
        handoff::artifact(&ledger, &roots, &target, stage)?,
        &mut body,
    );
    if let Target::Recording(id) = &target
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

fn flatten(mut copy: Value, body: &mut Value) {
    let identity = copy["identity"].take();
    for (key, value) in identity.as_object().into_iter().flatten() {
        body[key] = value.clone();
    }
    for key in ["title", "content", "provenance", "generated_at"] {
        body[key] = copy[key].take();
    }
}
