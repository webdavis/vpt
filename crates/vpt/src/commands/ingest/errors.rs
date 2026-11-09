use crate::compose::errors::{contained_error, ledger_error};
use vpt_adapters::ContainedError;
use vpt_application::IngestFailure;
use vpt_application::ports::SinkError;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn source(error: ContainedError) -> ErrorDocument {
    match error {
        ContainedError::Io { .. } => ErrorDocument::new(
            ErrorKind::Store,
            "Voice Memos recordings directory is unreadable",
        ),
        other => contained_error(other),
    }
}

pub(crate) fn document(failure: IngestFailure) -> ErrorDocument {
    match failure {
        IngestFailure::Ledger(error)
        | IngestFailure::RequiredHistory(error)
        | IngestFailure::SinkStopped(SinkError::RequiredHistory(error)) => ledger_error(error),
        IngestFailure::StoreUnreadable(_) | IngestFailure::Recorder(_) => {
            ErrorDocument::new(ErrorKind::Store, "Voice Memos source is unreadable")
        }
        IngestFailure::EmptyStore => ErrorDocument::new(
            ErrorKind::Store,
            "previously populated Voice Memos source is empty",
        ),
        IngestFailure::NoSpace => {
            ErrorDocument::new(ErrorKind::Store, "no space available for audio archive")
        }
        IngestFailure::Sync(_) => {
            ErrorDocument::new(ErrorKind::Store, "audio archive synchronization failed")
        }
        IngestFailure::ArchiveCollision { target, .. } => ErrorDocument::new(
            ErrorKind::Refused,
            format!("different content occupies {}", target.display()),
        )
        .rule("archive_collision"),
        IngestFailure::OnceMissing(_) => ErrorDocument::new(
            ErrorKind::Usage,
            "requested source is not an eligible recording",
        ),
        IngestFailure::PathEscape(_) => ErrorDocument::new(
            ErrorKind::Refused,
            "source or archive path escapes its checked authority",
        )
        .rule("path_escape"),
        IngestFailure::HelperVersion { .. } => ErrorDocument::new(
            ErrorKind::Refused,
            "vpt-macos has an incompatible major version",
        )
        .rule("helper_version"),
        IngestFailure::SinkStopped(SinkError::UnsafeState) => {
            ErrorDocument::new(ErrorKind::Refused, "state directory is unsafe").rule("unsafe_state")
        }
        IngestFailure::Archive(_) => {
            ErrorDocument::new(ErrorKind::Store, "audio archive operation failed")
        }
    }
}
