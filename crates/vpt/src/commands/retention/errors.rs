use crate::compose::errors::{ledger_error, store_error};
use vpt_application::RetentionError;
use vpt_application::ports::{SinkError, TrashError};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn document(error: RetentionError) -> ErrorDocument {
    match error {
        RetentionError::Disabled => ErrorDocument::new(ErrorKind::Config, "retention is disabled"),
        RetentionError::Ledger(error) | RetentionError::RequiredHistory(error) => {
            ledger_error(error)
        }
        RetentionError::Stores(error) => store_error(error),
        RetentionError::TargetModified(path) => ErrorDocument::new(
            ErrorKind::Refused,
            format!("retention target was modified: {}", path.display()),
        )
        .rule("retention_target_modified"),
        RetentionError::MissingHistory { .. } => ErrorDocument::new(
            ErrorKind::Refused,
            "retention intent has no surviving history",
        )
        .rule("missing_history"),
        RetentionError::SinkStopped(SinkError::UnsafeState) => {
            ErrorDocument::new(ErrorKind::Refused, "state directory is unsafe").rule("unsafe_state")
        }
        RetentionError::SinkStopped(SinkError::RequiredHistory(error)) => ledger_error(error),
        RetentionError::Trash(TrashError::HelperAbsent) => ErrorDocument::new(
            ErrorKind::Refused,
            "vpt-macos is unavailable; install the helper to move files to Trash",
        )
        .rule("helper_absent"),
        RetentionError::Trash(TrashError::HelperVersion { .. }) => ErrorDocument::new(
            ErrorKind::Refused,
            "vpt-macos has an incompatible major version",
        )
        .rule("helper_version"),
        RetentionError::Trash(TrashError::Failed(_)) => {
            ErrorDocument::new(ErrorKind::Helper, "Trash move failed")
        }
        RetentionError::Trash(TrashError::Unknown(_)) => ErrorDocument::new(
            ErrorKind::Helper,
            "Trash move outcome is unknown; retention will reconcile it on the next run",
        ),
    }
}
