use crate::cli::output::Outcome;
use crate::compose::{Environment, settings_from};
use serde_json::json;
use std::path::{Path, PathBuf};
use vpt_adapters::{SymlinkError, deploy_symlink, verify_symlink};
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

fn link_and_target(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<(PathBuf, PathBuf), Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let target = settings.symlink_target.ok_or_else(|| {
        Box::new(ErrorDocument::new(
            ErrorKind::Usage,
            "home.symlink_target is not set",
        ))
    })?;
    if !settings.home.is_absolute() || !target.is_absolute() {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "home.path and home.symlink_target must be absolute",
        )));
    }
    Ok((settings.home, target))
}

fn describe(error: &SymlinkError, link: &Path, target: &Path) -> String {
    match error {
        SymlinkError::Occupied(what) => format!("{} exists and is {what}", link.display()),
        SymlinkError::TargetParentMissing => {
            format!("the parent of {} does not exist", target.display())
        }
        SymlinkError::TargetNotDirectory => format!("{} is not a directory", target.display()),
        SymlinkError::TargetMissing => format!("{} does not exist", target.display()),
        SymlinkError::Missing => format!("{} does not exist", link.display()),
        SymlinkError::WrongTarget(existing) => format!(
            "{} points at {} rather than {}",
            link.display(),
            existing.display(),
            target.display()
        ),
        SymlinkError::Io(detail) => detail.clone(),
    }
}

fn outcome(
    verb: &str,
    rule: &str,
    result: Result<(), SymlinkError>,
    link: &Path,
    target: &Path,
) -> Outcome {
    match result {
        Ok(()) => Outcome::Success {
            completed: Vec::new(),
            diagnostics: Vec::new(),
            human: format!("{} -> {}\n", link.display(), target.display()),
            document: document(verb, json!({"link": link, "target": target, "ok": true})),
        },
        Err(SymlinkError::Io(detail)) => {
            Outcome::Failure(ErrorDocument::new(ErrorKind::Store, detail))
        }
        Err(error) => Outcome::Failure(
            ErrorDocument::new(ErrorKind::Refused, describe(&error, link, target)).rule(rule),
        ),
    }
}

pub(crate) fn deploy(environment: &Environment, config: Option<&Path>) -> Outcome {
    match link_and_target(environment, config) {
        Ok((link, target)) => outcome(
            "symlink deploy",
            "symlink_deploy",
            deploy_symlink(&link, &target),
            &link,
            &target,
        ),
        Err(error) => Outcome::Failure(*error),
    }
}

pub(crate) fn verify(environment: &Environment, config: Option<&Path>) -> Outcome {
    match link_and_target(environment, config) {
        Ok((link, target)) => outcome(
            "symlink verify",
            "symlink_verify",
            verify_symlink(&link, &target),
            &link,
            &target,
        ),
        Err(error) => Outcome::Failure(*error),
    }
}
