use super::errors;
use crate::cli::output::Outcome;
use serde_json::Value;
use std::path::Path;
use vpt_adapters::{Roots, SystemClock};
use vpt_application::ports::Clock;
use vpt_domain::{digest::Sha256Digest, handoff::*};
use vpt_protocol::error::ErrorDocument;
pub(crate) struct Artifact<'a> {
    pub kind: Kind,
    pub identity: Identity<'a>,
    pub path: &'a Path,
    pub digest: Sha256Digest,
    pub body: &'a str,
    pub provenance: Provenance<'a>,
}
pub(crate) fn value(roots: &Roots, artifact: Artifact<'_>) -> Result<Value, Box<ErrorDocument>> {
    let path = artifact
        .path
        .strip_prefix(&roots.home)
        .unwrap_or(artifact.path);
    let document = Document {
        kind: artifact.kind,
        identity: artifact.identity,
        note: path.to_str().ok_or_else(errors::changed)?,
        digest: artifact.digest,
        stem: artifact
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(errors::changed)?,
        body: artifact.body,
        provenance: artifact.provenance,
        generated_at: SystemClock.now(),
    };
    vpt_protocol::handoff::value(&document).map_err(errors::document)
}
pub(super) fn outcome(value: Value) -> Result<Outcome, Box<ErrorDocument>> {
    let human = value["content"]
        .as_str()
        .ok_or_else(errors::changed)?
        .to_owned();
    Ok(Outcome::Success {
        document: value,
        human,
        completed: vec![],
        diagnostics: vec![],
    })
}
