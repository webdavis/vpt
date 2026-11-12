mod execute;
mod output;
mod scan;
use crate::{
    cli::output::Outcome,
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_domain::tags::{RenameError, Vocabulary};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) struct Request<'a> {
    pub old: &'a str,
    pub new: &'a str,
    pub dry_run: bool,
}

pub(crate) fn rename(
    environment: &Environment,
    config: Option<&Path>,
    request: &Request<'_>,
) -> Outcome {
    prepare(environment, config, request).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn prepare(
    environment: &Environment,
    config: Option<&Path>,
    request: &Request<'_>,
) -> Result<Outcome, Box<ErrorDocument>> {
    Vocabulary::new(&[])
        .expect("empty vocabulary")
        .rename(request.old, request.new)
        .map_err(usage)?;
    let settings = settings_from(environment, config)?;
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
    if request.dry_run {
        scan::preview(&settings, &roots, request)
    } else {
        execute::run(&settings, &roots, request)
    }
}

fn usage(error: RenameError) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Usage,
        match error {
            RenameError::Syntax => "tag names must contain bounded, valid tag text",
            RenameError::Same => "the new tag name equals the old one",
        },
    ))
}
