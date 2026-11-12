mod proposal;
mod redacted;
mod segments;
pub(crate) mod stage;
use crate::cli::output::Outcome;
use crate::documents::observation_json;
use vpt_application::ports::ObservationLedger;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
use vpt_protocol::result::document;

pub(crate) fn run(ledger: &dyn ObservationLedger, id: &str) -> Outcome {
    let Ok(identity) = RecordingId::parse(id) else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "invalid recording id"));
    };
    match ledger.observe_recordings(Some(&identity)) {
        Ok(records) if !records.is_empty() => {
            let record = &records[0];
            let json = match observation_json(record) {
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
                completed: Vec::new(),
                diagnostics: Vec::new(),
                human,
                document: document("show", json),
            }
        }
        Ok(_) => Outcome::Failure(
            ErrorDocument::new(ErrorKind::Usage, "recording not found").ids(vec![id.to_owned()]),
        ),
        Err(error) => Outcome::Failure(crate::compose::errors::ledger_error(error)),
    }
}

fn timecode(seconds: u64) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
