use crate::{cli::output::Outcome, compose::events::attach};
use std::collections::BTreeSet;
use vpt_application::ports::SinkWarning;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn finish(
    result: Result<Outcome, Box<ErrorDocument>>,
    warnings: BTreeSet<SinkWarning>,
    id: &RecordingId,
    committed: bool,
    retention: Option<crate::compose::startup::StartupReport>,
) -> Outcome {
    let mut outcome = attach(result, &warnings);
    if committed {
        outcome.add_completed([id.to_string()]);
    }
    if let Some(retention) = retention {
        outcome = crate::compose::startup::attach(outcome, retention);
    }
    outcome
}

pub(crate) fn success(
    record: &vpt_application::ports::RecordingRecord,
    target: std::path::PathBuf,
) -> Result<Outcome, Box<ErrorDocument>> {
    let record = crate::documents::record_json(record).map_err(Box::new)?;
    Ok(Outcome::Success {
        completed: Vec::new(),
        diagnostics: Vec::new(),
        document: vpt_protocol::result::document(
            "note write",
            serde_json::json!({"record": record, "paths": [target]}),
        ),
        human: format!("Wrote {}\n", target.display()),
    })
}
