use super::super::cloning::path_error;
use super::*;
use std::io::ErrorKind;

#[test]
fn destination_creation_errors_preserve_classes_without_private_path_text() {
    let path = Path::new("/owned/private-recording-name");
    for (kind, expected) in [
        (ErrorKind::StorageFull, CloneError::NoSpace),
        (ErrorKind::AlreadyExists, CloneError::Exists),
        (
            ErrorKind::PermissionDenied,
            CloneError::Io(ErrorKind::PermissionDenied.to_string()),
        ),
        (
            ErrorKind::NotFound,
            CloneError::Io(ErrorKind::NotFound.to_string()),
        ),
    ] {
        assert_eq!(
            path_error(
                path,
                ContainedError::Io {
                    path: path.into(),
                    kind
                }
            ),
            expected
        );
    }
    for error in [
        ContainedError::NotRegular(path.into()),
        ContainedError::NotADirectory(path.into()),
        ContainedError::Escape {
            root: "/owned".into(),
            path: path.into(),
        },
    ] {
        assert_eq!(path_error(path, error), CloneError::Escape(path.into()));
    }
}

#[test]
fn recorder_errors_preserve_typed_refusals_and_safe_io_text() {
    let path = Path::new("/owned/private-recording-name");
    for (error, expected) in [
        (
            ContainedError::NotRegular(path.into()),
            RecorderError::NotRegular(path.into()),
        ),
        (
            ContainedError::NotADirectory(path.into()),
            RecorderError::Escape(path.into()),
        ),
        (
            ContainedError::Escape {
                root: "/owned".into(),
                path: path.into(),
            },
            RecorderError::Escape(path.into()),
        ),
        (
            ContainedError::Io {
                path: path.into(),
                kind: ErrorKind::NotFound,
            },
            RecorderError::NotFound(path.into()),
        ),
        (
            ContainedError::Io {
                path: path.into(),
                kind: ErrorKind::PermissionDenied,
            },
            RecorderError::Io(ErrorKind::PermissionDenied.to_string()),
        ),
    ] {
        assert_eq!(recorder_error(path, error), expected);
    }
}

#[test]
fn listing_errors_distinguish_unreadable_roots_from_containment_refusals() {
    let path = Path::new("/owned/private-root-name");
    for kind in [
        ErrorKind::PermissionDenied,
        ErrorKind::NotFound,
        ErrorKind::Other,
    ] {
        assert_eq!(
            listing_error(
                path,
                ContainedError::Io {
                    path: path.into(),
                    kind
                }
            ),
            RecorderError::Unreadable(kind.to_string())
        );
    }
    for error in [
        ContainedError::NotRegular(path.into()),
        ContainedError::NotADirectory(path.into()),
        ContainedError::Escape {
            root: "/owned".into(),
            path: path.into(),
        },
    ] {
        assert_eq!(
            listing_error(path, error),
            RecorderError::Escape(path.into())
        );
    }
}
