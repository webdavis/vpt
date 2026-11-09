use vpt_protocol::error::{ErrorDocument, ErrorKind};
pub(super) fn usage(message: &str) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(ErrorKind::Usage, message))
}
pub(super) fn read() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Store,
        "note could not be read safely",
    ))
}
pub(super) fn ownership() -> Box<ErrorDocument> {
    Box::new(
        ErrorDocument::new(ErrorKind::Refused, "note belongs to a different recording")
            .rule("ownership"),
    )
}
pub(super) fn content() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Store,
        "note content cannot be verified within the supported limits",
    ))
}
