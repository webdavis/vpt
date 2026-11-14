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
fn an_edit_kept_beside_the_note_names_both_files_and_which_version_each_holds() {
    let error = store_error(StoreError::EditKept(KeptEdit {
        note: "/owned/private-canary/note.md".into(),
        kept: "/owned/private-canary/.note.md.vpt-kept-7-2".into(),
        versions: KeptVersions::OlderSaveAtNote,
        cause: None,
    }));
    assert_eq!(error.kind, ErrorKind::Refused);
    assert_eq!(error.rule.as_deref(), Some("publication_edit_kept"));
    assert_eq!(
        error.message,
        "note.md changed while vpt published it; it holds your older save and your newer save \
         is kept beside it as .note.md.vpt-kept-7-2; keep what you need in note.md, then delete \
         .note.md.vpt-kept-7-2"
    );
    assert_eq!(error.exit_code(), 3);
    assert!(!error.to_json().to_string().contains("private-canary"));
}

#[test]
fn a_missing_note_tells_the_person_to_rename_the_kept_file_and_keeps_the_cause() {
    let error = store_error(StoreError::EditKept(KeptEdit {
        note: "/owned/private-canary/note.md".into(),
        kept: "/owned/private-canary/.note.md.vpt-kept-7-2".into(),
        versions: KeptVersions::NoteMissing,
        cause: Some(Box::new(StoreError::Io("permission denied".into()))),
    }));
    assert_eq!(error.rule.as_deref(), Some("publication_edit_kept"));
    assert_eq!(
        error.message,
        "note.md changed while vpt published it and is now missing; its last version is kept \
         beside it as .note.md.vpt-kept-7-2; rename .note.md.vpt-kept-7-2 to note.md; vpt could \
         not put it back: permission denied"
    );
}

#[test]
fn a_disk_that_cannot_replace_a_file_safely_is_a_named_refusal() {
    let error = store_error(StoreError::UnsafeVolume(
        "/owned/private-canary/note.md".into(),
    ));
    assert_eq!(error.kind, ErrorKind::Refused);
    assert_eq!(error.rule.as_deref(), Some("publication_volume_unsafe"));
    assert_eq!(
        error.message,
        "vpt did not publish note.md: the disk that holds its folder cannot replace a file \
         safely; set this store to a folder on another disk"
    );
    assert!(!error.to_json().to_string().contains("private-canary"));
}

#[test]
fn operational_errors_retain_ledger_or_store_exit_classes() {
    assert_eq!(
        root_error(&RootError::UnresolvedParent {
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
