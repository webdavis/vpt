use vpt_application::review::resolve::ResolutionError;
use vpt_domain::review::ResolveError;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn usage(message: &str) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(ErrorKind::Usage, message))
}

pub(super) fn document(error: ResolutionError) -> Box<ErrorDocument> {
    match error {
        ResolutionError::UnknownFlag => usage("flag not found"),
        ResolutionError::InvalidSnapshot => Box::new(ErrorDocument::new(
            ErrorKind::Ledger,
            "invalid transcript flags",
        )),
        ResolutionError::InvalidTerm | ResolutionError::Action(ResolveError::InvalidText) => {
            usage("correction must contain bounded, valid text")
        }
        ResolutionError::Action(ResolveError::HiddenFlag) => {
            usage("formatting flags cannot be resolved")
        }
        ResolutionError::Action(ResolveError::DiagnosticCorrection) => {
            usage("diagnostics can only be confirmed or dismissed")
        }
    }
}
