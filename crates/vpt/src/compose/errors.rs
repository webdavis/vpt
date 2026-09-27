use vpt_adapters::{ContainedError, OpenError, RootError};
use vpt_application::ports::{LedgerError, StoreError};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn root_error(error: &RootError) -> ErrorDocument {
    let message = match error {
        RootError::NotAbsolute { key } => format!("{key} must be absolute after expansion"),
        RootError::ParentMissing { key } => format!("{key}: the parent directory does not exist"),
        RootError::NotADirectory { key } => format!("{key}: not a directory"),
        RootError::PathEscape { key } => {
            return ErrorDocument::new(
                ErrorKind::Refused,
                format!("{key}: path escapes its resolved root"),
            )
            .rule("path_escape");
        }
        RootError::Overlap { first, second } => format!("{first} overlaps {second}"),
        RootError::Io { key, detail } => format!("{key}: {detail}"),
    };
    ErrorDocument::new(ErrorKind::Config, message)
}

pub(crate) fn contained_error(error: ContainedError) -> ErrorDocument {
    match error {
        ContainedError::Io { kind, .. } => {
            ErrorDocument::new(ErrorKind::Store, format!("{kind} at checked path"))
        }
        ContainedError::Escape { .. }
        | ContainedError::NotRegular(_)
        | ContainedError::NotADirectory(_) => {
            ErrorDocument::new(ErrorKind::Refused, "path escapes a checked root")
                .rule("path_escape")
        }
    }
}

pub(crate) fn open_error(error: OpenError) -> ErrorDocument {
    match error {
        OpenError::Contained(error) => contained_error(error),
        OpenError::Ledger(error) => ledger_error(error),
    }
}

pub(crate) fn ledger_error(error: LedgerError) -> ErrorDocument {
    match error {
        LedgerError::PathEscape(_) => ErrorDocument::new(
            ErrorKind::Refused,
            "ledger path escapes its retained state root",
        )
        .rule("path_escape"),
        LedgerError::Busy => ErrorDocument::new(ErrorKind::Ledger, "ledger is busy"),
        LedgerError::UnsupportedSchema(_) => {
            ErrorDocument::new(ErrorKind::Ledger, "unsupported ledger schema")
        }
        LedgerError::Io(_) => ErrorDocument::new(ErrorKind::Ledger, "ledger operation failed"),
        LedgerError::Conflict(_) => ErrorDocument::new(ErrorKind::Ledger, "ledger conflict"),
        LedgerError::Corrupt(_) => ErrorDocument::new(ErrorKind::Ledger, "ledger is corrupt"),
    }
}

pub(crate) fn store_error(error: StoreError) -> ErrorDocument {
    match error {
        StoreError::Escape(_) => {
            ErrorDocument::new(ErrorKind::Refused, "store path escapes its checked root")
                .rule("path_escape")
        }
        StoreError::Io(detail) => ErrorDocument::new(ErrorKind::Store, detail),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn path() -> PathBuf {
        "/owned/private-canary".into()
    }

    fn assert_refused(error: ErrorDocument) {
        assert_eq!(error.kind, ErrorKind::Refused);
        assert_eq!(error.rule.as_deref(), Some("path_escape"));
    }

    #[test]
    fn path_escapes_are_typed_refusals() {
        assert_refused(root_error(&RootError::PathEscape {
            key: "home.state_dir".into(),
        }));
        assert_refused(contained_error(ContainedError::Escape {
            root: "/owned".into(),
            path: path(),
        }));
        assert_refused(open_error(OpenError::Ledger(LedgerError::PathEscape(
            path(),
        ))));
        assert_refused(ledger_error(LedgerError::PathEscape(path())));
        assert_refused(store_error(StoreError::Escape(path())));
    }

    #[test]
    fn operational_errors_retain_ledger_or_store_exit_classes() {
        assert_eq!(
            root_error(&RootError::ParentMissing {
                key: "home.state_dir".into()
            })
            .kind,
            ErrorKind::Config
        );
        assert_eq!(
            contained_error(ContainedError::Io {
                path: path(),
                kind: std::io::ErrorKind::PermissionDenied,
            })
            .kind,
            ErrorKind::Store
        );
        assert_eq!(
            open_error(OpenError::Ledger(LedgerError::Corrupt("bad".into()))).kind,
            ErrorKind::Ledger
        );
        assert_eq!(ledger_error(LedgerError::Busy).kind, ErrorKind::Ledger);
        assert_eq!(
            store_error(StoreError::Io("read failed".into())).kind,
            ErrorKind::Store
        );
    }
}
