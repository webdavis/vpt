use vpt_application::SetupError;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn config_error(error: impl std::fmt::Debug) -> ErrorDocument {
    ErrorDocument::new(ErrorKind::Config, format!("{error:?}"))
}

pub(crate) fn setup_error(error: SetupError) -> ErrorDocument {
    match error {
        SetupError::NoTerminal => ErrorDocument::new(
            ErrorKind::Usage,
            "vpt setup needs a terminal to choose the engine; pass --engine apple or --engine \
             whisply",
        ),
        SetupError::Exists(path) => ErrorDocument::new(
            ErrorKind::Usage,
            format!("{} exists; pass --overwrite to replace it", path.display()),
        ),
        SetupError::PathEscape(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("{} escapes its prepared root", path.display()),
        )
        .rule("path_escape"),
        SetupError::NonPrivateConfig(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!(
                "{} must already be private (mode 0700) and owned by the current user",
                path.display()
            ),
        )
        .rule("nonprivate_config_directory"),
        SetupError::NonPrivateDirectory(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!(
                "{} must already be private (mode 0700) and owned by the current user",
                path.display()
            ),
        )
        .rule("nonprivate_directory"),
        SetupError::MultiplyLinkedConfig(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("{} has multiple hard links", path.display()),
        )
        .rule("multiply_linked_config"),
        SetupError::Io(detail) => ErrorDocument::new(ErrorKind::Store, detail),
    }
}
