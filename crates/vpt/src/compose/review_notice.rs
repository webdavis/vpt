use vpt_application::{
    ports::ReviewLedger,
    review::{self, alerts::ReviewNotice},
};
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn read(
    ledger: &impl ReviewLedger,
    id: &RecordingId,
    path: std::path::PathBuf,
) -> Result<Option<ReviewNotice>, Box<ErrorDocument>> {
    let review = review::review(ledger, id).map_err(|error| {
        Box::new(match error {
            review::ReviewError::Ledger(error) => super::errors::ledger_error(error),
            _ => ErrorDocument::new(ErrorKind::Ledger, "published review state is unavailable"),
        })
    })?;
    Ok(ReviewNotice::from_review(&review, path))
}
