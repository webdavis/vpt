use super::{INSTALL_HINT, checks::check};
use crate::compose::{RuntimeLedger, errors};
use std::path::Path;
use vpt_adapters::config::planned_directory;
use vpt_adapters::{
    BUILT_AGAINST_MAJOR, ClonefileArchive, ContainedError, HelperClient, HelperError, LinkState,
    RootDir, enclosing_git_tree, inspect_symlink, verify_symlink,
};
use vpt_application::Settings;
use vpt_application::ports::{Archive, RecordingLedger, SeenRow};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::{Check, ErrorDocument};

pub(super) fn git_tree(key: StoreKey, path: &Path) -> Check {
    let name = format!("git_tree:{}", key.key_name());
    match enclosing_git_tree(path) {
        Ok(None) => check(name, true, "not in a git tree"),
        Ok(Some(tree)) => {
            let note = match key {
                StoreKey::Audio => "; confirm the ignore rules exclude audio",
                StoreKey::Drafts | StoreKey::EngineOutputs => {
                    "; sensitive: this store holds private JSON"
                }
                _ => "",
            };
            check(
                name,
                true,
                format!("inside git tree {}{note}", tree.display()),
            )
        }
        Err(error) => check(
            name,
            false,
            format!("git ancestry could not be inspected: {}", error.kind()),
        ),
    }
}

pub(super) fn helper(settings: &Settings) -> Check {
    let helper = HelperClient::new(settings.helper_path.clone());
    let mut result = match helper.version() {
        Ok(version) => check(
            "helper",
            true,
            format!("vpt-macos {version} at {}", settings.helper_path.display()),
        ),
        Err(HelperError::Absent) => check(
            "helper",
            false,
            format!(
                "{} not found; {INSTALL_HINT}",
                settings.helper_path.display()
            ),
        ),
        Err(HelperError::MajorMismatch { found }) => check(
            "helper",
            false,
            format!("major version {found}, this build needs {BUILT_AGAINST_MAJOR}"),
        ),
        Err(HelperError::Failed(detail) | HelperError::Unknown(detail)) => {
            check("helper", false, detail)
        }
    };
    for diagnostic in helper.take_diagnostics() {
        result.detail.push_str(&format!("; {diagnostic}"));
    }
    result
}

pub(super) fn subdirectory(recordings: &Path, name: &str) -> Check {
    let result = RootDir::open(recordings).and_then(|root| root.open_subdirectory(name));
    let label = format!("subdirectory:{name}");
    match result {
        Ok(root) => match root.names() {
            Ok(names) => check(label, true, format!("{} entries", names.len())),
            Err(error) => check(label, false, errors::contained_error(error).message),
        },
        Err(ContainedError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) => check(label, true, "absent"),
        Err(error) => check(label, false, errors::contained_error(error).message),
    }
}

pub(super) fn symlink(settings: &Settings) -> Check {
    match &settings.symlink_target {
        Some(target) => match verify_symlink(&settings.home, target) {
            Ok(()) => check(
                "symlink",
                true,
                format!("{} -> {}", settings.home.display(), target.display()),
            ),
            Err(error) => check("symlink", false, format!("{error:?}")),
        },
        None => match inspect_symlink(&settings.home) {
            LinkState::LinkTo(target) => check(
                "symlink",
                true,
                format!(
                    "followed, not managed: {} -> {}",
                    settings.home.display(),
                    target.display()
                ),
            ),
            LinkState::Unreadable(detail) => check("symlink", false, detail),
            _ => check("symlink", true, "not configured"),
        },
    }
}

pub(super) fn cleanup_pending(audio: &Path) -> Check {
    let archive = match ClonefileArchive::open_read_only(audio) {
        Ok(archive) => archive,
        Err(error) => {
            return check(
                "cleanup_pending",
                false,
                errors::contained_error(error).message,
            );
        }
    };
    match archive.staged_leftovers() {
        Ok(leftovers) if leftovers.is_empty() => check("cleanup_pending", true, "none"),
        Ok(leftovers) => {
            let names = leftovers
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            check(
                "cleanup_pending",
                false,
                format!(
                    "{} staged files await the Trash: {}",
                    names.len(),
                    names.join(", ")
                ),
            )
        }
        Err(_) => check(
            "cleanup_pending",
            false,
            "staged files could not be inspected",
        ),
    }
}

pub(super) fn source_gone(state: &Path) -> Check {
    let rows = planned_directory(state, "home.state_dir")
        .map_err(|error| Box::new(errors::root_error(&error)))
        .and_then(|path| RuntimeLedger::read_only(&path))
        .and_then(|ledger| {
            ledger
                .seen_all()
                .map_err(|error| Box::new(errors::ledger_error(error)))
        });
    source_result(rows)
}

fn source_result(rows: Result<Vec<SeenRow>, Box<ErrorDocument>>) -> Check {
    match rows {
        Ok(rows) => check(
            "source_gone",
            true,
            format!(
                "{} recordings whose source is gone",
                rows.iter()
                    .filter(|row| row.source_gone_at.is_some())
                    .count()
            ),
        ),
        Err(error) => check("source_gone", false, error.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vpt_application::ports::LedgerError;

    fn assert_error(error: LedgerError, expected: &str) {
        let result = source_result(Err(Box::new(crate::compose::errors::ledger_error(error))));
        assert_eq!(
            result,
            Check {
                name: "source_gone".into(),
                ok: false,
                notice: false,
                detail: expected.into()
            }
        );
        assert!(!result.detail.contains("private-canary"));
    }

    #[test]
    fn source_gone_busy_is_safe() {
        assert_error(LedgerError::Busy, "ledger is busy");
    }
    #[test]
    fn source_gone_schema_is_safe() {
        assert_error(
            LedgerError::UnsupportedSchema(4321),
            "unsupported ledger schema",
        );
    }
    #[test]
    fn source_gone_io_is_safe() {
        assert_error(
            LedgerError::Io("private-canary".into()),
            "ledger operation failed",
        );
    }
    #[test]
    fn source_gone_conflict_is_safe() {
        assert_error(
            LedgerError::Conflict("private-canary".into()),
            "ledger conflict",
        );
    }
    #[test]
    fn source_gone_corrupt_is_safe() {
        assert_error(
            LedgerError::Corrupt("private-canary".into()),
            "ledger is corrupt",
        );
    }
    #[test]
    fn source_gone_escape_is_safe() {
        assert_error(
            LedgerError::PathEscape("/owned/private-canary".into()),
            "ledger path escapes its retained state root",
        );
    }
}
