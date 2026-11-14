use super::probes;
use crate::compose::{Environment, settings_from};
use std::path::Path;
use vpt_adapters::APPLE_SUBDIRECTORIES;
use vpt_adapters::config::{planned_directory, resolve};
use vpt_domain::layout::StoreKey;
use vpt_protocol::error::Check;

pub(super) fn check(name: impl Into<String>, ok: bool, detail: impl Into<String>) -> Check {
    Check {
        name: name.into(),
        ok,
        notice: false,
        detail: detail.into(),
    }
}

pub(super) fn notice(name: impl Into<String>, detail: &str) -> Check {
    Check {
        notice: true,
        ..check(name, true, detail)
    }
}

fn unavailable_config(detail: &str) -> Vec<Check> {
    let mut names: Vec<String> = ["config", "stores", "vault"].map(str::to_owned).into();
    names.extend(StoreKey::all().map(|key| format!("git_tree:{}", key.key_name())));
    names.extend(["helper", "engines", "recordings_dir", "symlink"].map(str::to_owned));
    names.extend(APPLE_SUBDIRECTORIES.map(|name| format!("subdirectory:{name}")));
    names.extend(["cleanup_pending", "kept_edits", "source_gone"].map(str::to_owned));
    names
        .into_iter()
        .map(|name| {
            let detail = if name == "config" {
                format!("{detail}; run `vpt setup`")
            } else {
                "not run: the configuration did not load".into()
            };
            check(name, false, detail)
        })
        .collect()
}

pub(super) fn all(environment: &Environment, config: Option<&Path>) -> Vec<Check> {
    let selected = match config
        .map(Path::to_path_buf)
        .map_or_else(|| environment.config_path(), Ok)
    {
        Ok(selected) => selected,
        Err(_) => return unavailable_config("configuration path is unavailable"),
    };
    let settings = match settings_from(environment, Some(&selected)) {
        Ok(settings) => settings,
        Err(error) => return unavailable_config(&error.message),
    };
    let mut checks = vec![check(
        "config",
        true,
        format!("config_version {}", settings.config_version),
    )];
    let config_dir = selected
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let roots = resolve(&settings, config_dir);
    checks.push(super::vault::inspect(&settings));
    checks.push(match &roots {
        Ok(roots) => check("stores", true, format!("home {}", roots.home.display())),
        Err(error) => check(
            "stores",
            false,
            crate::compose::errors::root_error(error).message,
        ),
    });
    checks.extend(StoreKey::all().map(|key| {
        let path = match &roots {
            Ok(roots) => Ok(roots.stores.get(key).to_path_buf()),
            Err(_) => planned_directory(
                settings.stores.get(key),
                &format!("stores.{}", key.key_name()),
            ),
        };
        match path {
            Ok(path) => probes::git_tree(key, &path),
            Err(error) => check(
                format!("git_tree:{}", key.key_name()),
                false,
                crate::compose::errors::root_error(&error).message,
            ),
        }
    }));
    let helper = probes::helper(&settings);
    checks.push(super::engines::inspect(&settings.engines, helper.ok));
    checks.push(helper);
    let (recordings, subdirectories) = super::source::inspect(&settings.source.recordings_dir);
    checks.push(recordings);
    checks.push(probes::symlink(&settings));
    checks.extend(subdirectories);
    checks.push(match &roots {
        Ok(roots) => probes::cleanup_pending(&roots.stores.audio),
        Err(_) => check(
            "cleanup_pending",
            false,
            "not run: store roots did not resolve",
        ),
    });
    checks.push(match &roots {
        Ok(roots) => super::kept::inspect(roots, &settings.state_dir),
        Err(_) => check("kept_edits", false, "not run: store roots did not resolve"),
    });
    checks.push(probes::source_gone(&settings.state_dir));
    checks
}
