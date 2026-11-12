use super::errors;
use serde_json::Value;
use std::path::Path;
use vpt_adapters::{Roots, SystemClock};
use vpt_application::ports::Clock;
use vpt_domain::{copy::*, digest::Sha256Digest};
use vpt_protocol::error::ErrorDocument;
pub(crate) struct Artifact<'a> {
    pub kind: Kind,
    pub identity: Identity<'a>,
    pub path: &'a Path,
    pub digest: Sha256Digest,
    pub body: &'a str,
    pub provenance: Provenance<'a>,
    pub withhold_path: bool,
}
pub(crate) fn value(roots: &Roots, artifact: Artifact<'_>) -> Result<Value, Box<ErrorDocument>> {
    let path = artifact
        .path
        .strip_prefix(&roots.home)
        .unwrap_or(artifact.path);
    let id = artifact.identity.as_str().to_owned();
    let (note, stem) = match artifact.withhold_path {
        true => (id.as_str(), id.as_str()),
        false => (
            path.to_str().ok_or_else(errors::changed)?,
            artifact
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(errors::changed)?,
        ),
    };
    let document = Document {
        kind: artifact.kind,
        identity: artifact.identity,
        note,
        digest: artifact.digest,
        stem,
        body: artifact.body,
        provenance: artifact.provenance,
        generated_at: SystemClock.now(),
    };
    let mut value = vpt_protocol::copy::fields(&document).map_err(errors::document)?;
    if artifact.withhold_path {
        value["note"] = Value::Null;
        value["provenance"]["redacted"] = true.into();
    }
    Ok(value)
}
