mod resolve;
pub(crate) use resolve::run as resolve;
mod output;
use crate::cli::output::Outcome;
use vpt_application::{
    ports::ReviewLedger,
    review::{ReviewError, review},
};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(ledger: &impl ReviewLedger, id: &str) -> Outcome {
    let Ok(identity) = RecordingId::parse(id) else {
        return Outcome::Failure(ErrorDocument::new(ErrorKind::Usage, "invalid recording id"));
    };
    load(ledger, &identity).unwrap_or_else(|error| Outcome::Failure(*error))
}

fn load(ledger: &impl ReviewLedger, identity: &RecordingId) -> Result<Outcome, Box<ErrorDocument>> {
    review(ledger, identity)
        .map(|review| output::success(&review))
        .map_err(|error| {
            Box::new(match error {
                ReviewError::NotFound => {
                    ErrorDocument::new(ErrorKind::Usage, "recording not found")
                        .ids(vec![identity.to_string()])
                }
                ReviewError::InvalidFlags => {
                    ErrorDocument::new(ErrorKind::Ledger, "invalid transcript flags")
                }
                ReviewError::Ledger(error) => crate::compose::errors::ledger_error(error),
            })
        })
}
