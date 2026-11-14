use super::errors::{contained_error, root_error};
use super::{Environment, RuntimeLedger, settings_from};
use std::path::Path;
use vpt_adapters::FilesystemStores;
use vpt_adapters::config::{planned_directory, resolve_stores};
use vpt_application::StorePaths;
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::ErrorDocument;

pub(crate) fn ledger_from(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<RuntimeLedger, Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let state = planned_directory(&settings.state_dir, "home.state_dir")
        .map_err(|error| Box::new(root_error(&error)))?;
    RuntimeLedger::read_only(&state)
}

pub(crate) fn storage_from(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<(StorePaths, FilesystemStores), Box<ErrorDocument>> {
    let settings = settings_from(environment, config)?;
    let paths = resolve_stores(&settings).map_err(|error| Box::new(root_error(&error)))?;
    let configured = StoreKey::all().map(|key| paths.get(key).to_path_buf());
    let stores = FilesystemStores::open_read_only(&configured)
        .map_err(|error| Box::new(contained_error(error)))?;
    Ok((paths, stores))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    fn fixture() -> (tempfile::TempDir, Environment, PathBuf) {
        let temp = crate::test_temp::tempdir().unwrap();
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let home = temp.path().join("home");
        let config = temp.path().join("config.toml");
        std::fs::create_dir(&home).unwrap();
        std::fs::write(&config, "config_version = 1\n").unwrap();
        std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600)).unwrap();
        let environment = Environment {
            vars: vec![(OsString::from("HOME"), home.as_os_str().to_owned())],
        };
        (temp, environment, config)
    }

    #[test]
    fn ledger_composition_reads_missing_state_without_creation() {
        let (temp, environment, config) = fixture();
        assert!(ledger_from(&environment, Some(&config)).is_ok());
        assert!(!temp.path().join("home/.local/state/vpt").exists());
    }

    #[test]
    fn storage_composition_reads_missing_stores_without_creation() {
        let (temp, environment, config) = fixture();
        assert!(storage_from(&environment, Some(&config)).is_ok());
        assert!(!temp.path().join("home/.vpt").exists());
    }

    #[test]
    fn storage_composition_refuses_a_regular_file_root() {
        let (temp, environment, config) = fixture();
        let regular = temp.path().join("regular-audio");
        std::fs::write(&regular, b"not a directory").unwrap();
        std::fs::write(
            &config,
            format!("config_version = 1\n[stores]\naudio = {regular:?}\n"),
        )
        .unwrap();
        match storage_from(&environment, Some(&config)) {
            Err(error) => assert_eq!(error.kind, vpt_protocol::error::ErrorKind::Config),
            Ok(_) => panic!("regular file was accepted as a store root"),
        }
    }
}
