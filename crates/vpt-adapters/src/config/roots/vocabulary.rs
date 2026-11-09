use super::{RootError, Roots};
use std::path::{Component, Path};
use vpt_application::Settings;

pub(super) fn validate(settings: &Settings, roots: &Roots) -> Result<(), RootError> {
    let terms = &settings.reconcile.known_terms_path;
    let tags = &settings.tags.known_tags_path;
    validate_path(terms, "reconcile.known_terms_path", roots)?;
    validate_path(tags, "tags.known_tags_path", roots)?;
    if tags.starts_with(terms) || terms.starts_with(tags) {
        return Err(RootError::Overlap {
            first: "tags.known_tags_path".into(),
            second: "reconcile.known_terms_path".into(),
        });
    }
    Ok(())
}

fn validate_path(path: &Path, key: &str, roots: &Roots) -> Result<(), RootError> {
    if !path.is_absolute() {
        return Err(RootError::NotAbsolute { key: key.into() });
    }
    if path.file_name().is_none()
        || path
            .to_str()
            .is_none_or(|text| text.len() > 65_536 || text.chars().any(char::is_control))
        || path
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(RootError::PathEscape { key: key.into() });
    }
    for (protected, name) in [
        (&roots.state_dir, "home.state_dir"),
        (&roots.container, "source.recordings_dir"),
        (&roots.stores.released, "stores.released"),
    ] {
        if path.starts_with(protected) || protected.starts_with(path) {
            return Err(RootError::Overlap {
                first: key.into(),
                second: name.into(),
            });
        }
    }
    Ok(())
}
