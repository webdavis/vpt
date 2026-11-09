use std::os::unix::fs::MetadataExt;
use std::path::Path;
use vpt_application::Settings;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn protect_config(settings: &Settings, config: &Path) -> Result<(), Box<ErrorDocument>> {
    for (target, key) in [
        (
            &settings.reconcile.known_terms_path,
            "reconcile.known_terms_path",
        ),
        (&settings.tags.known_tags_path, "tags.known_tags_path"),
    ] {
        if same_location(target, config) {
            return Err(Box::new(ErrorDocument::new(
                ErrorKind::Config,
                format!("{key} overlaps the configuration file"),
            )));
        }
    }
    if same_location(
        &settings.tags.known_tags_path,
        &settings.reconcile.known_terms_path,
    ) {
        return Err(Box::new(ErrorDocument::new(
            ErrorKind::Config,
            "tags.known_tags_path overlaps reconcile.known_terms_path",
        )));
    }
    Ok(())
}

fn same_location(left: &Path, right: &Path) -> bool {
    let same_file = match (left.metadata(), right.metadata()) {
        (Ok(left), Ok(right)) => (left.dev(), left.ino()) == (right.dev(), right.ino()),
        _ => false,
    };
    left == right
        || same_file
        || (left
            .canonicalize()
            .ok()
            .is_some_and(|left| right.canonicalize().ok().as_ref() == Some(&left)))
}
