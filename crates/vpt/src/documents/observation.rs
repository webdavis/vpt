use super::record::{checked_path, record_json};
use serde_json::{Value, json};
use vpt_application::ports::{ObservedFlag, RecordingObservation, RetentionStatus};
use vpt_protocol::error::ErrorDocument;

#[allow(
    clippy::result_large_err,
    reason = "CLI outcomes own the error document directly"
)]
pub(crate) fn observation_json(row: &RecordingObservation) -> Result<Value, ErrorDocument> {
    let mut value = record_json(&row.record)?;
    let transcript = row.transcription.as_ref();
    value["engines"] = json!(
        transcript
            .map(|t| t
                .engines
                .iter()
                .map(|engine| json!({
                    "name": engine.name, "tool": engine.tool, "version": engine.version,
                    "model": engine.model, "family": engine.family,
                }))
                .collect::<Vec<_>>())
            .unwrap_or_default()
    );
    value["language"] = json!(transcript.map(|t| &t.language));
    value["requested_language"] = json!(transcript.map(|t| &t.requested_language));
    value["language_source"] = json!(transcript.map(|t| match t.language_source {
        vpt_domain::transcript::LanguageSource::Reported => "reported",
        vpt_domain::transcript::LanguageSource::Requested => "requested",
    }));
    value["local"] = json!(transcript.map(|t| t.local));
    value["from_checker"] = json!(transcript.map(|t| t.from_checker));
    value["open_flags"] = json!(row.flags.len());
    value["diagnostics"] = json!(row.diagnostics.len());
    value["flags"] = json!(row.flags.iter().map(flag_json).collect::<Vec<_>>());
    value["diagnostic_flags"] = json!(row.diagnostics.iter().map(flag_json).collect::<Vec<_>>());
    value["stale_verification"] = json!(
        row.stale_artifacts
            .iter()
            .map(|stage| stage.as_str())
            .collect::<Vec<_>>()
    );
    let mut paths = json!({"transcript": null, "analysis": null});
    let mut artifacts = Vec::with_capacity(row.artifacts.len());
    for artifact in &row.artifacts {
        let path = checked_path(&artifact.path)?;
        match artifact.kind {
            vpt_application::ports::ArtifactKind::Transcript => paths["transcript"] = json!(path),
            vpt_application::ports::ArtifactKind::Analysis => paths["analysis"] = json!(path),
            _ => {}
        }
        artifacts.push(json!({
            "kind": artifact.kind.as_str(), "path": path, "digest": artifact.digest.hex(),
            "status": match artifact.status {
                RetentionStatus::Published => "published",
                RetentionStatus::PublicationPending => "publication_pending",
                RetentionStatus::Expired => "expired",
            },
            "trashed_at": artifact.trashed_at.map(|at| at.rfc3339()),
        }));
    }
    value["note_paths"] = paths;
    value["artifacts"] = json!(artifacts);
    Ok(value)
}

fn flag_json(flag: &ObservedFlag) -> Value {
    json!({"identifier": flag.identifier.hex(), "class": flag.class, "artifact": flag.artifact.map(|stage| stage.as_str())})
}
