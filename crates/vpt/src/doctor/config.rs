use super::report::Row;
use crate::compose::{Environment, settings_from};
use std::path::{Path, PathBuf};
use vpt_application::Settings;

pub(super) type Loaded = Result<(PathBuf, Settings), String>;

pub(super) fn selected(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<PathBuf, String> {
    config
        .map(Path::to_path_buf)
        .map_or_else(|| environment.config_path(), Ok)
        .map_err(|_| "configuration path is unavailable".to_string())
}

pub(super) fn load(environment: &Environment, config: Option<&Path>) -> Loaded {
    let selected = selected(environment, config)?;
    let settings = settings_from(environment, Some(&selected)).map_err(|error| error.message)?;
    Ok((selected, settings))
}

pub(super) fn missing(environment: &Environment, config: Option<&Path>) -> bool {
    selected(environment, config).is_ok_and(|path| {
        path.symlink_metadata()
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
    })
}

pub(super) fn directory(selected: &Path) -> &Path {
    selected
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

pub(super) fn rows(environment: &Environment, config: Option<&Path>, loaded: &Loaded) -> Vec<Row> {
    match loaded {
        Ok((selected, settings)) => vec![
            Row::ok("file", selected.display().to_string()),
            Row::ok(
                "version",
                format!("config_version {}", settings.config_version),
            ),
        ],
        Err(message) if missing(environment, config) => vec![Row::issue("file", message).advice(
            "run `vpt doctor --fix` to create the starter configuration, or `vpt setup` to \
                 choose the engine",
        )],
        Err(message) => vec![Row::issue("file", message).advice("run `vpt setup`")],
    }
}
