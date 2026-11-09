use crate::cli::output::Outcome;
use std::path::PathBuf;
use vpt_application::ports::RecordingRecord;
use vpt_protocol::error::ErrorDocument;

pub(super) fn success(
    record: &RecordingRecord,
    engine: &str,
    language: &str,
    engines: &[serde_json::Value],
    paths: &[PathBuf],
    diagnostics: &[&str],
    engine_diagnostics: &[String],
) -> Result<Outcome, Box<ErrorDocument>> {
    let mut json = crate::documents::record_json(record).map_err(Box::new)?;
    json["engine"] = engine.into();
    json["language"] = language.into();
    json["engines"] = serde_json::json!(engines);
    json["engine_outputs"] = serde_json::json!(paths);
    json["diagnostics"] = serde_json::json!(diagnostics);
    json["engine_diagnostics"] = serde_json::json!(engine_diagnostics);
    let details: String = engine_diagnostics
        .iter()
        .map(|detail| format!("Engine: {detail}\n"))
        .collect();
    Ok(Outcome::Success {
        completed: vec![record.id.to_string()],
        diagnostics: diagnostics
            .iter()
            .map(|s| (*s).to_owned())
            .chain(engine_diagnostics.iter().cloned())
            .collect(),
        document: vpt_protocol::result::document("transcribe", json),
        human: format!(
            "Transcribed {} using {} ({})\n{details}",
            record.id, engine, language
        ),
    })
}

pub(super) fn engine_diagnostics(
    result: &vpt_application::transcribe::Transcription,
) -> Vec<String> {
    let mut diagnostics: Vec<_> = result
        .failures
        .iter()
        .map(super::super::errors::attempt)
        .collect();
    for output in std::iter::once(&result.record).chain(result.checker.as_ref()) {
        diagnostics.extend(
            output
                .diagnostics
                .iter()
                .map(|detail| format!("{}: {detail}", output.transcript.engine.name)),
        );
    }
    diagnostics
}

pub(super) fn engines(
    result: &vpt_application::transcribe::Transcription,
) -> Vec<serde_json::Value> {
    std::iter::once(&result.record)
        .chain(result.checker.as_ref())
        .map(|output| {
            serde_json::json!({
                "name": output.transcript.engine.name,
                "family": output.transcript.engine.family,
                "local": output.transcript.local,
            })
        })
        .collect()
}
