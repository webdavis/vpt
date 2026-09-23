mod directory;

pub use directory::planned_directory;
use directory::{private_dir, prospective};
use std::path::{Component, Path, PathBuf};
use vpt_application::{Settings, StorePaths};
use vpt_domain::layout::{RootName, StoreKey, check_overlaps};

#[derive(Debug, PartialEq, Eq)]
pub enum RootError {
    NotAbsolute { key: String },
    ParentMissing { key: String },
    NotADirectory { key: String },
    PathEscape { key: String },
    Overlap { first: String, second: String },
    Io { key: String, detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roots {
    pub home: PathBuf,
    pub state_dir: PathBuf,
    pub stores: StorePaths,
    pub recordings_dir: PathBuf,
    pub container: PathBuf,
    pub config_dir: PathBuf,
}

pub fn resolve(settings: &Settings, config_dir: &Path) -> Result<Roots, RootError> {
    let home = planned_directory(&settings.home, "home.path")?;
    let state_dir = planned_directory(&settings.state_dir, "home.state_dir")?;
    let recordings_dir =
        planned_directory(&settings.source.recordings_dir, "source.recordings_dir")?;
    let container = recordings_dir
        .parent()
        .ok_or_else(|| RootError::NotAbsolute {
            key: "source.recordings_dir".into(),
        })?
        .to_path_buf();
    let config_path = if config_dir.is_absolute() {
        config_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| io("config", error))?
            .join(config_dir)
    };
    let config_dir = planned_directory(&config_path, "config")?;
    let stores = store_paths(settings, &home)?;
    let mut roots = vec![
        (RootName::Home, home.as_path()),
        (RootName::State, state_dir.as_path()),
        (RootName::Config, config_dir.as_path()),
        (RootName::VoiceMemos, container.as_path()),
    ];
    roots.extend(
        StoreKey::all()
            .iter()
            .map(|key| (RootName::Store(*key), stores.get(*key))),
    );
    validate_overlaps(&roots)?;
    Ok(Roots {
        home,
        state_dir,
        stores,
        recordings_dir,
        container,
        config_dir,
    })
}

pub fn resolve_stores(settings: &Settings) -> Result<StorePaths, RootError> {
    let home = planned_directory(&settings.home, "home.path")?;
    let stores = store_paths(settings, &home)?;
    let roots = StoreKey::all()
        .iter()
        .map(|key| (RootName::Store(*key), stores.get(*key)))
        .collect::<Vec<_>>();
    validate_overlaps(&roots)?;
    Ok(stores)
}

fn store_paths(settings: &Settings, home: &Path) -> Result<StorePaths, RootError> {
    let mut stores = settings.stores.clone();
    for key in StoreKey::all() {
        let configured = settings.stores.get(key);
        let below_home = configured
            .strip_prefix(&settings.home)
            .ok()
            .and_then(|rest| {
                let mut components = rest.components();
                match (components.next(), components.next()) {
                    (Some(Component::Normal(name)), None) => Some(home.join(name)),
                    _ => None,
                }
            });
        stores.set(
            key,
            prospective(
                configured,
                &format!("stores.{}", key.key_name()),
                below_home,
            )?,
        );
    }
    Ok(stores)
}

fn validate_overlaps(roots: &[(RootName, &Path)]) -> Result<(), RootError> {
    check_overlaps(roots).map_err(|conflict| RootError::Overlap {
        first: conflict.first.key_name(),
        second: conflict.second.key_name(),
    })
}

fn io(key: &str, error: std::io::Error) -> RootError {
    RootError::Io {
        key: key.into(),
        detail: error.kind().to_string(),
    }
}

impl Roots {
    pub fn create_state_dir(&self) -> Result<(), RootError> {
        private_dir(&self.state_dir, "home.state_dir")
    }

    pub fn create_leaves(&self) -> Result<(), RootError> {
        private_dir(&self.home, "home.path")?;
        for key in StoreKey::all() {
            private_dir(self.stores.get(key), &format!("stores.{}", key.key_name()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
