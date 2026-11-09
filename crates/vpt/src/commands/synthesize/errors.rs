use vpt_application::ports::SynthesisFailure;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn usage(message: &str) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(ErrorKind::Usage, message))
}
pub(super) fn invalid() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Command,
        "synthesis output cannot be grounded or rendered",
    ))
}
pub(super) fn changed() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(
            ErrorKind::Refused,
            "synthesis source changed during the command",
        )
        .rule("synthesis_input_changed"),
    )
}
pub(super) fn command(error: SynthesisFailure) -> Box<ErrorDocument> {
    let interrupted = matches!(error, SynthesisFailure::Interrupted);
    let message = match error {
        SynthesisFailure::Disabled => "synthesis command is not configured",
        SynthesisFailure::InvalidArguments => "synthesis command arguments are invalid",
        SynthesisFailure::InputLimit => "synthesis input exceeds its limit",
        SynthesisFailure::Unavailable => "synthesis command is unavailable",
        SynthesisFailure::Io => "synthesis command I/O failed",
        SynthesisFailure::Exited(_) => "synthesis command exited unsuccessfully",
        SynthesisFailure::Signaled(_) => "synthesis command ended by signal",
        SynthesisFailure::DeadlineExceeded => "synthesis command exceeded its deadline",
        SynthesisFailure::Interrupted => "synthesis command was interrupted",
        SynthesisFailure::OutputLimit => "synthesis output exceeds its limit",
        SynthesisFailure::UnsupportedVersion(version) => {
            return Box::new(ErrorDocument::new(
                ErrorKind::Command,
                format!("unsupported synthesis output schema vpt.proposal/{version}"),
            ));
        }
        SynthesisFailure::InvalidDocument => "synthesis output document is invalid",
    };
    let mut document = ErrorDocument::new(ErrorKind::Command, message);
    document.interrupted = interrupted;
    Box::new(document)
}
