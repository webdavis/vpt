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
fn changed_publication_is_a_named_refusal_without_private_path_text() {
    let error = store_error(StoreError::Changed(path()));
    assert_eq!(error.kind, ErrorKind::Refused);
    assert_eq!(error.rule.as_deref(), Some("publication_target_modified"));
    assert_eq!(error.exit_code(), 3);
    assert!(!error.to_json().to_string().contains("private-canary"));
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
