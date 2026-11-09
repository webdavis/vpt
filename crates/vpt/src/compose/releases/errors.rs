use vpt_adapters::ReleaseFileError;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn document(error: ReleaseFileError) -> Box<ErrorDocument> {
    Box::new(match error {
        ReleaseFileError::Store(error) => crate::compose::errors::store_error(error),
        ReleaseFileError::Protected(root) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("release destination resolves inside {root}"),
        )
        .rule("release_destination_private"),
        ReleaseFileError::Duplicate(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("identical release already exists at {}", path.display()),
        )
        .rule("duplicate"),
        ReleaseFileError::PrivateReport => ErrorDocument::new(
            ErrorKind::Refused,
            "release report must be an owned mode-0600 file with one link",
        )
        .rule("release_report_private"),
        ReleaseFileError::Invalid => ErrorDocument::new(
            ErrorKind::Refused,
            "release paths or saved content are invalid",
        )
        .rule("release_invalid"),
        ReleaseFileError::Capacity => ErrorDocument::new(
            ErrorKind::Refused,
            "release exceeds bounded file or content capacity",
        )
        .rule("release_capacity"),
    })
}
