use super::fix::{Action, Repair};
use crate::compose::mutation;
use std::path::Path;
use std::time::Duration;
use vpt_adapters::{ClonefileArchive, HelperClient, Roots};
use vpt_application::Settings;
use vpt_application::ports::{Archive, Trash, TrashError};

pub(super) fn staged_audio(settings: &Settings, roots: &Roots) -> Vec<Repair> {
    let Ok(archive) = ClonefileArchive::open_read_only(&roots.stores.audio) else {
        return Vec::new();
    };
    if archive
        .staged_leftovers()
        .map_or(true, |found| found.is_empty())
    {
        return Vec::new();
    }
    let locked =
        mutation::state(roots).and_then(|state| mutation::lock_with_wait(&state, Duration::ZERO));
    let _lock = match locked {
        Ok(lock) => lock,
        Err(error) => return vec![repair("any staged file".into(), Err(error.message))],
    };
    let Ok(staged) = archive.staged_leftovers() else {
        return Vec::new();
    };
    let helper = HelperClient::new(settings.helper_path.clone());
    staged
        .iter()
        .map(|path| repair(path.display().to_string(), trash(&archive, &helper, path)))
        .collect()
}

fn trash(archive: &ClonefileArchive, helper: &HelperClient, path: &Path) -> Result<(), String> {
    archive
        .validate_trash_path(path)
        .map_err(|_| "it is no longer a regular file in the audio store".to_string())?;
    helper.trash(path).map(drop).map_err(|error| match error {
        TrashError::HelperAbsent => "vpt-macos is unavailable".into(),
        TrashError::HelperVersion { .. } => "vpt-macos has an incompatible major version".into(),
        TrashError::Failed(_) => "the Trash move failed".into(),
        TrashError::Unknown(_) => "the Trash move outcome is unknown".into(),
    })
}

fn repair(path: String, made: Result<(), String>) -> Repair {
    Repair {
        check: "leftovers",
        name: "staged_audio",
        action: Action::Trash(path),
        made,
    }
}
