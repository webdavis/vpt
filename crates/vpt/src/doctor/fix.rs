use super::checks::named;
use super::config;
use crate::commands::{setup, symlink};
use crate::compose::Environment;
use crate::report::Check;
use crate::style::Verdict;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use vpt_adapters::config::{create_private_directory, resolve};
use vpt_adapters::{LinkState, deploy_symlink, inspect_symlink, verify_symlink};
use vpt_application::Settings;
use vpt_domain::layout::StoreKey;

pub(super) struct Repair {
    pub(super) check: &'static str,
    pub(super) name: &'static str,
    pub(super) action: Action,
    pub(super) made: Result<(), String>,
}

pub(super) enum Action {
    Create,
    Trash(String),
}

pub(super) fn repair(
    environment: &Environment,
    config: Option<&Path>,
    wanted: &[String],
) -> Vec<Repair> {
    let mut repairs = Vec::new();
    if named(wanted, "config") && config::missing(environment, config) {
        repairs.push(Repair {
            check: "config",
            name: "file",
            action: Action::Create,
            made: setup::write_starter(environment, config)
                .map(drop)
                .map_err(|error| error.message),
        });
    }
    let Ok((selected, settings)) = config::load(environment, config) else {
        return repairs;
    };
    if named(wanted, "stores") {
        repairs.extend(home_link(&settings));
        repairs.extend(directories(&settings, config::directory(&selected)));
    }
    if named(wanted, "leftovers")
        && let Ok(roots) = resolve(&settings, config::directory(&selected))
    {
        repairs.extend(super::trash::staged_audio(&settings, &roots));
    }
    repairs
}

fn home_link(settings: &Settings) -> Option<Repair> {
    let link = settings.home.as_path();
    let target = settings.symlink_target.as_deref()?;
    let absent = matches!(inspect_symlink(link), LinkState::Absent);
    (absent && link.is_absolute() && target.is_absolute()).then(|| Repair {
        check: "stores",
        name: "home_link",
        action: Action::Create,
        made: deploy_symlink(link, target).map_err(|error| symlink::describe(&error, link, target)),
    })
}

fn directories(settings: &Settings, config_dir: &Path) -> Vec<Repair> {
    let Ok(roots) = resolve(settings, config_dir) else {
        return Vec::new();
    };
    let unlinked = settings
        .symlink_target
        .as_deref()
        .is_some_and(|target| verify_symlink(&settings.home, target).is_err());
    let mut planned: Vec<(&'static str, PathBuf)> = vec![
        ("home", roots.home.clone()),
        ("state_dir", roots.state_dir.clone()),
    ];
    planned.extend(StoreKey::all().map(|key| (key.key_name(), roots.stores.get(key).into())));
    planned
        .into_iter()
        .filter(|(_, path)| !(unlinked && path.starts_with(&roots.home)))
        .filter(|(_, path)| {
            path.symlink_metadata()
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        })
        .map(|(name, path)| Repair {
            check: "stores",
            name,
            action: Action::Create,
            made: create_private_directory(&path)
                .map_err(|error| setup::setup_error(error).message),
        })
        .collect()
}

pub(super) fn mark(checks: &mut [Check], repairs: Vec<Repair>) -> Vec<Value> {
    let mut fixed = Vec::new();
    for repair in repairs {
        let row = checks
            .iter_mut()
            .filter(|check| check.name == repair.check)
            .flat_map(|check| check.rows.iter_mut())
            .find(|row| row.name == repair.name);
        let Some(row) = row else { continue };
        let detail = match (repair.made, repair.action) {
            (Ok(()), Action::Create) => {
                row.detail = format!("created {}", row.detail);
                row.detail.clone()
            }
            (Ok(()), Action::Trash(path)) => format!("moved {path} to the Trash"),
            (Err(reason), action) => {
                let target = match action {
                    Action::Create => "create it".to_string(),
                    Action::Trash(path) => format!("move {path} to the Trash"),
                };
                row.verdict = Verdict::Issue;
                row.detail = format!("{}; could not {target}: {reason}", row.detail);
                continue;
            }
        };
        fixed.push(json!({"check": repair.check, "name": repair.name, "detail": detail}));
    }
    fixed
}
