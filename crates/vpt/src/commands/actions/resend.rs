mod mutation;
use super::{list, rows};
use crate::{
    cli::output::Outcome,
    compose::{Environment, RuntimeLedger, actions::require_command, settings_from},
};
use std::path::Path;
use vpt_adapters::Roots;
use vpt_application::{Settings, ports::SendState};
use vpt_domain::identity::RecordingId;
use vpt_protocol::{error::ErrorDocument, result::document};

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    id: Option<&str>,
    preview: bool,
) -> Outcome {
    execute(environment, config, id, preview).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn execute(
    environment: &Environment,
    config: Option<&Path>,
    id: Option<&str>,
    preview: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let only = rows::recording(id)?;
    let settings = settings_from(environment, config)?;
    require_command(&settings)?;
    let config = config.map(Path::to_path_buf).map_or_else(
        || {
            environment
                .config_path()
                .map_err(crate::compose::config_error)
        },
        Ok,
    )?;
    let roots = vpt_adapters::config::resolve(&settings, config.parent().unwrap_or(Path::new(".")))
        .map_err(|error| Box::new(crate::compose::errors::root_error(&error)))?;
    if preview {
        return planned(&settings, &roots, only.as_ref());
    }
    mutation::run(&settings, &roots, only.as_ref())
}

/// Lists what a resend would send or withdraw, and changes nothing.
fn planned(
    settings: &Settings,
    roots: &Roots,
    only: Option<&RecordingId>,
) -> Result<Outcome, Box<ErrorDocument>> {
    let ledger = RuntimeLedger::read_only(&roots.state_dir)?;
    let mut states = vec![SendState::Refused, SendState::Pending];
    if !settings.actions_withdraw_command.is_empty() {
        states.push(SendState::Withdrawing);
    }
    let rows = rows::read(&ledger, only, &states)?;
    let human = if rows.is_empty() {
        "nothing to send\n".into()
    } else {
        list::human(&rows)
    };
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        human,
        document: document(
            "actions resend",
            serde_json::json!({"preview": true, "to_send": rows.iter().map(rows::Row::json).collect::<Vec<_>>()}),
        ),
    })
}
