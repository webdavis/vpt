use crate::cli::output::Outcome;
use crate::documents::record_json;
use vpt_application::ports::RecordingLedger;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub(crate) fn run(ledger: &dyn RecordingLedger, id: &str) -> Outcome {
    let Ok(identity) = RecordingId::parse(id) else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "invalid recording id"));
    };
    match ledger.by_id(&identity) {
        Ok(Some(record)) => {
            let json = match record_json(&record) {
                Ok(json) => json,
                Err(error) => return Outcome::Failure(error),
            };
            let human = match serde_json::to_string_pretty(&json) {
                Ok(text) => format!("{text}\n"),
                Err(_) => {
                    return Outcome::Failure(ErrorDocument::new(
                        ErrorKind::Ledger,
                        "record cannot be serialized",
                    ));
                }
            };
            Outcome::Success {
                human,
                document: document("show", json),
            }
        }
        Ok(None) => Outcome::Failure(
            ErrorDocument::new(ErrorKind::Usage, "recording not found").ids(vec![id.to_owned()]),
        ),
        Err(error) => Outcome::Failure(crate::compose::errors::ledger_error(error)),
    }
}
