use vpt_protocol::error::{ErrorDocument, ErrorKind};
pub(super) fn missing() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Usage,
        "recording has no source artifact for the requested stage",
    ))
}
pub(super) fn source() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(
            ErrorKind::Refused,
            "redaction source ownership, content, or saved flags changed",
        )
        .rule("redact_source"),
    )
}
pub(super) fn ledger(error: vpt_application::ports::LedgerError) -> Box<ErrorDocument> {
    Box::new(crate::compose::errors::ledger_error(error))
}
pub(super) fn redaction(error: vpt_domain::redaction::Error) -> Box<ErrorDocument> {
    use vpt_domain::redaction::Error;
    Box::new(match error {
        Error::Residue { class, number } => {
            let label = number.map_or_else(
                || class.as_str().to_owned(),
                |n| format!("[{} {n}]", class.as_str()),
            );
            ErrorDocument::new(ErrorKind::Refused, format!("redaction residue for {label}"))
                .rule("residue")
        }
        Error::Capacity => {
            ErrorDocument::new(ErrorKind::Refused, "redaction exceeds content capacity")
                .rule("redact_capacity")
        }
        Error::InvalidMask => ErrorDocument::new(ErrorKind::Config, "invalid share.redact.mask"),
        Error::InvalidText => return source(),
    })
}
