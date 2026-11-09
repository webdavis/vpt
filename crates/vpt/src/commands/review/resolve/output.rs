use super::Request;
use crate::cli::output::Outcome;
use std::path::Path;
use vpt_application::ports::ReviewLedger;
use vpt_domain::review::ReviewAction;
use vpt_protocol::error::ErrorDocument;

pub(super) fn success(
    ledger: &impl ReviewLedger,
    request: &Request<'_>,
    path: Option<&Path>,
    changed: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let state = match request.action {
        ReviewAction::Confirm => "confirmed",
        ReviewAction::Correct(_) => "corrected",
        ReviewAction::Dismiss => "dismissed",
    };
    let mut outcome = super::super::load(ledger, &request.id)?;
    if let Outcome::Success {
        document, human, ..
    } = &mut outcome
    {
        document["resolved"] = serde_json::json!({
            "identifier": request.flag.hex(), "state": state, "changed": changed, "path": path,
        });
        human.insert_str(0, &format!("Resolved {}: {state}\n", request.flag.hex()));
    }
    Ok(outcome)
}
