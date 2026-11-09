use vpt_domain::run::RunOutcome;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn from_result<T>(result: &Result<T, Box<ErrorDocument>>) -> RunOutcome {
    match result {
        Ok(_) => RunOutcome::Completed,
        Err(error) if error.interrupted => RunOutcome::Interrupted,
        Err(_) => RunOutcome::Failed,
    }
}
