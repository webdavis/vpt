use vpt_application::ports::LedgerError;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
pub(super) fn ledger(error: LedgerError) -> Box<ErrorDocument> {
    Box::new(crate::compose::errors::ledger_error(error))
}
pub(super) fn missing() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Usage,
        "no existing artifact for this identity and stage",
    ))
}
pub(super) fn private() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(
            ErrorKind::Refused,
            "private handoff requires handoff.allow_private",
        )
        .rule("private_handoff"),
    )
}
pub(super) fn changed() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(ErrorKind::Refused, "artifact bytes or ownership changed")
            .rule("artifact_changed"),
    )
}
pub(super) fn pending() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Store,
        "artifact publication is pending",
    ))
}
pub(super) fn document(error: vpt_domain::handoff::Error) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Store,
        match error {
            vpt_domain::handoff::Error::Invalid => "invalid handoff metadata",
            vpt_domain::handoff::Error::CapacityExceeded => "handoff capacity exceeded",
        },
    ))
}
