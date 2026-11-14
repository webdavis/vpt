use std::path::Path;
use vpt_adapters::{ContainedError, OpenError, RootError};
use vpt_application::ports::{KeptEdit, KeptVersions, LedgerError, StoreError};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn root_error(error: &RootError) -> ErrorDocument {
    let message = match error {
        RootError::NotAbsolute { key } => format!("{key} must be absolute after expansion"),
        RootError::UnresolvedParent { key } => {
            format!("{key}: a .. below a missing directory cannot be resolved")
        }
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
        StoreError::Changed(_) => ErrorDocument::new(
            ErrorKind::Refused,
            "pending publication target was modified",
        )
        .rule("publication_target_modified"),
        StoreError::EditKept(edit) => {
            ErrorDocument::new(ErrorKind::Refused, kept_message(edit)).rule("publication_edit_kept")
        }
        StoreError::UnsafeVolume(note) => ErrorDocument::new(
            ErrorKind::Refused,
            format!(
                "vpt did not publish {}: the disk that holds its folder cannot replace a file \
                 safely; set this store to a folder on another disk",
                file_name(&note)
            ),
        )
        .rule("publication_volume_unsafe"),
        StoreError::Io(detail) => ErrorDocument::new(ErrorKind::Store, detail),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn kept_message(edit: KeptEdit) -> String {
    let (note, kept) = (file_name(&edit.note), file_name(&edit.kept));
    let both = |versions: &str| {
        format!(
            "{note} changed while vpt published it; {versions} beside it as {kept}; keep what you \
             need in {note}, then delete {kept}"
        )
    };
    let message = match edit.versions {
        KeptVersions::PublishedAtNote => {
            both("it holds vpt's new version and the version vpt replaced is kept")
        }
        KeptVersions::NewerSaveAtNote => {
            both("it holds your newer save and your older save is kept")
        }
        KeptVersions::OlderSaveAtNote => {
            both("it holds your older save and your newer save is kept")
        }
        KeptVersions::EditedNote => both("it holds an edited version and another version is kept"),
        KeptVersions::NoteMissing => format!(
            "{note} changed while vpt published it and is now missing; its last version is kept \
             beside it as {kept}; rename {kept} to {note}"
        ),
    };
    match edit.cause {
        Some(cause) => format!(
            "{message}; vpt could not put it back: {}",
            store_error(*cause).message
        ),
        None => message,
    }
}

#[cfg(test)]
mod tests;
