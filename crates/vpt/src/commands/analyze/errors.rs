use vpt_application::ports::AnalysisFailure;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn usage(message: &str) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(ErrorKind::Usage, message))
}
pub(super) fn invalid() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Command,
        "analysis output cannot be grounded or rendered",
    ))
}
pub(super) fn changed() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(
            ErrorKind::Refused,
            "analysis source changed during the command",
        )
        .rule("analysis_input_changed"),
    )
}
pub(super) fn command(error: AnalysisFailure) -> Box<ErrorDocument> {
    let interrupted = matches!(error, AnalysisFailure::Interrupted);
    let message = match error {
        AnalysisFailure::Disabled => "analysis command is not configured",
        AnalysisFailure::InvalidArguments => "analysis command arguments are invalid",
        AnalysisFailure::InputLimit => "analysis input exceeds its limit",
        AnalysisFailure::Unavailable => "analysis command is unavailable",
        AnalysisFailure::Io => "analysis command I/O failed",
        AnalysisFailure::Exited(_) => "analysis command exited unsuccessfully",
        AnalysisFailure::Signaled(_) => "analysis command ended by signal",
        AnalysisFailure::DeadlineExceeded => "analysis command exceeded its deadline",
        AnalysisFailure::Interrupted => "analysis command was interrupted",
        AnalysisFailure::OutputLimit => "analysis output exceeds its limit",
        AnalysisFailure::UnsupportedVersion(version) => {
            return Box::new(ErrorDocument::new(
                ErrorKind::Command,
                format!("unsupported analysis output schema vpt.proposal/{version}"),
            ));
        }
        AnalysisFailure::InvalidDocument => "analysis output document is invalid",
    };
    let mut document = ErrorDocument::new(ErrorKind::Command, message);
    document.interrupted = interrupted;
    Box::new(document)
}
