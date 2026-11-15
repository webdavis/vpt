mod errors;
mod plan;

use crate::cli::output::Outcome;
use crate::compose::Environment;
pub(crate) use errors::setup_error;
use serde_json::json;
use std::path::{Path, PathBuf};
use vpt_adapters::TtyPrompt;
use vpt_adapters::config::template;
use vpt_application::ports::{Prompt, PromptError};
use vpt_application::{Setup, SetupError, SetupWriter};
use vpt_protocol::error::ErrorDocument;
use vpt_protocol::result::document;

pub(crate) const ENGINES: [&str; 2] = ["apple", "whisply"];

pub(crate) fn run(
    environment: &Environment,
    config: Option<&Path>,
    overwrite: bool,
    engine: Option<&str>,
) -> Outcome {
    match prepare(environment, config, overwrite, engine) {
        Ok(outcome) => outcome,
        Err(error) => Outcome::Failure(error),
    }
}

#[allow(clippy::result_large_err, reason = "one-shot CLI result stays inline")]
fn prepare(
    environment: &Environment,
    config: Option<&Path>,
    overwrite: bool,
    engine: Option<&str>,
) -> Result<Outcome, ErrorDocument> {
    let plan = plan::plan(environment, config)?;
    let writer = plan.writer(&plan.directories)?;
    if writer.config_exists() && !overwrite {
        return Err(setup_error(SetupError::Exists(writer.config_path())));
    }
    let setup = Setup {
        engines: ENGINES.map(String::from).into(),
        render: |engine: &str| template(engine, &plan.state_dir),
        directories: plan.directories.clone(),
    };
    let outcome = match engine {
        Some(engine) => setup.run(&mut Chosen(engine), &writer, overwrite),
        None => setup.run(
            &mut TtyPrompt::open().map_err(|_| setup_error(SetupError::NoTerminal))?,
            &writer,
            overwrite,
        ),
    }
    .map_err(setup_error)?;
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: document(
            "setup",
            json!({"written": outcome.written, "main_engine": outcome.main_engine}),
        ),
        human: format!(
            "wrote {} (main engine {})\n",
            outcome.written.display(),
            outcome.main_engine
        ),
    })
}

#[allow(clippy::result_large_err, reason = "one-shot CLI result stays inline")]
pub(crate) fn write_starter(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<PathBuf, ErrorDocument> {
    let plan = plan::plan(environment, config)?;
    plan.writer(&[])?
        .write_config(&template(ENGINES[0], &plan.state_dir), false)
        .map_err(setup_error)
}

struct Chosen<'a>(&'a str);

impl Prompt for Chosen<'_> {
    fn choose(&mut self, _: &str, options: &[&str], _: usize) -> Result<usize, PromptError> {
        options
            .iter()
            .position(|option| *option == self.0)
            .ok_or(PromptError::Closed)
    }
}
