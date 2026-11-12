use vpt_application::ports::LedgerError;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
pub(crate) fn ledger(error: LedgerError) -> Box<ErrorDocument> {
    Box::new(crate::compose::errors::ledger_error(error))
}
pub(crate) fn missing() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Usage,
        "no existing artifact for this identity and stage",
    ))
}
pub(crate) fn private() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(
            ErrorKind::Refused,
            "private handoff requires handoff.allow_private",
        )
        .rule("private_handoff"),
    )
}
pub(crate) fn changed() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(ErrorKind::Refused, "artifact bytes or ownership changed")
            .rule("artifact_changed"),
    )
}
pub(crate) fn pending() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Store,
        "artifact publication is pending",
    ))
}
pub(crate) fn document(error: vpt_domain::handoff::Error) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Store,
        match error {
            vpt_domain::handoff::Error::Invalid => "invalid handoff metadata",
            vpt_domain::handoff::Error::CapacityExceeded => "handoff capacity exceeded",
        },
    ))
}
