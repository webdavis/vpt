mod execute;
mod mutation;
mod record;
mod relation;
mod tag;
use crate::{
    cli::output::Outcome,
    compose::{Environment, settings_from},
};
use std::path::Path;
use vpt_domain::{identity::RecordingId, tags::Action};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

enum Request<'a> {
    Term(&'a str),
    Tag(tag::Request<'a>),
    Relation(relation::Request<'a>),
}

impl Request<'_> {
    fn recording(&self) -> Option<&RecordingId> {
        match self {
            Self::Term(_) => None,
            Self::Tag(request) => Some(&request.id),
            Self::Relation(request) => Some(&request.id),
        }
    }
}

pub(crate) fn run_relation(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    value: &str,
) -> Outcome {
    relation::Request::parse(id, value)
        .and_then(|request| prepare(environment, config, Request::Relation(request)))
        .unwrap_or_else(|error| Outcome::Failure(*error))
}

pub(crate) fn run(environment: &Environment, config: Option<&Path>, term: &str) -> Outcome {
    prepare(environment, config, Request::Term(term))
        .unwrap_or_else(|error| Outcome::Failure(*error))
}

pub(crate) fn run_tag(
    environment: &Environment,
    config: Option<&Path>,
    id: &str,
    value: &str,
    action: Action,
) -> Outcome {
    let result = RecordingId::parse(id)
        .map_err(|_| usage("invalid recording id"))
        .and_then(|id| {
            prepare(
                environment,
                config,
                Request::Tag(tag::Request { id, value, action }),
            )
        });
    result.unwrap_or_else(|error| Outcome::Failure(*error))
}

fn usage(message: &str) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(ErrorKind::Usage, message))
}

fn prepare(
    environment: &Environment,
    config: Option<&Path>,
    request: Request<'_>,
) -> Result<Outcome, Box<ErrorDocument>> {
    match &request {
        Request::Term(term) if !vpt_domain::review::valid_correction(term) => {
            return Err(usage("term must contain bounded, valid text"));
        }
        Request::Tag(request) => tag::validate(request)?,
        _ => (),
    }
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
    if let Request::Tag(request) = &request {
        tag::preflight(request, &roots)?;
    }
    if let Request::Relation(request) = &request {
        relation::preflight(request, &settings, &roots)?;
    }
    mutation::run(&settings, &roots, &request)
}
