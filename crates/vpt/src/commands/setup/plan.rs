use super::errors::{config_error, setup_error};
use crate::compose::Environment;
use std::path::{Path, PathBuf};
use vpt_adapters::config::{FilesystemSetupWriter, from_table, load_text, resolve, template};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) struct Plan {
    pub(super) config: PathBuf,
    pub(super) state_dir: String,
    pub(super) directories: Vec<PathBuf>,
}

impl Plan {
    #[allow(clippy::result_large_err, reason = "one-shot CLI result stays inline")]
    pub(super) fn writer(
        &self,
        directories: &[PathBuf],
    ) -> Result<FilesystemSetupWriter, ErrorDocument> {
        FilesystemSetupWriter::new(self.config.clone(), directories).map_err(setup_error)
    }
}

#[allow(clippy::result_large_err, reason = "one-shot CLI result stays inline")]
pub(super) fn plan(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<Plan, ErrorDocument> {
    let selected = match config {
        Some(path) => path.to_path_buf(),
        None => environment.config_path().map_err(config_error)?,
    };
    let state_dir = environment.state_dir_default().map_err(config_error)?;
    let home_dir = environment.home_dir().map_err(config_error)?;
    let settings = load_text(&template("apple", &state_dir))
        .and_then(|table| from_table(&table, &home_dir))
        .map_err(config_error)?;
    let name = selected.file_name().ok_or_else(|| {
        ErrorDocument::new(ErrorKind::Usage, "the configuration path needs a file name")
    })?;
    let parent = selected
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let roots = resolve(&settings, parent).map_err(config_error)?;
    let mut directories = vec![roots.home.clone(), roots.state_dir.clone()];
    directories.extend(
        StoreKey::all()
            .iter()
            .map(|key| roots.stores.get(*key).to_path_buf()),
    );
    Ok(Plan {
        config: roots.config_dir.join(name),
        state_dir,
        directories,
    })
}
