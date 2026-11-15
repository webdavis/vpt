use super::checks::Context;
use crate::compose::errors::root_error;
use crate::report::Row;
use crate::style::Verdict;
use std::path::Path;
use vpt_adapters::SymlinkError;
use vpt_adapters::config::{planned_directory, resolve_vault};
use vpt_adapters::{LinkState, enclosing_git_tree, inspect_symlink, verify_symlink};
use vpt_application::Settings;
use vpt_domain::layout::StoreKey;

pub(super) fn rows(context: &Context) -> Vec<Row> {
    let settings = &context.settings;
    let mut rows = vec![
        planned("home", &settings.home, "home.path"),
        super::state_dir::row(&settings.state_dir),
        layout(context),
    ];
    rows.extend(StoreKey::all().map(|key| store(key, context)));
    rows.push(vault(settings));
    rows.push(home_link(settings));
    rows
}

fn planned(name: &str, path: &Path, key: &str) -> Row {
    match planned_directory(path, key) {
        Ok(path) => Row::ok(name, path.display().to_string()),
        Err(error) => Row::issue(name, root_error(&error).message),
    }
}

fn layout(context: &Context) -> Row {
    match (&context.roots, resolve_vault(&context.settings)) {
        (Ok(_), _) => Row::ok("layout", "every folder resolves and no two overlap"),
        (Err(_), Err(_)) => Row::not_run("layout", "the vault did not resolve"),
        (Err(error), Ok(_)) => Row::issue("layout", root_error(error).message),
    }
}

fn store(key: StoreKey, context: &Context) -> Row {
    let name = key.key_name();
    let path = match &context.roots {
        Ok(roots) => Ok(roots.stores.get(key).to_path_buf()),
        Err(_) => planned_directory(context.settings.stores.get(key), &format!("stores.{name}")),
    };
    let path = match path {
        Ok(path) => path,
        Err(error) => return Row::issue(name, root_error(&error).message),
    };
    let shown = path.display();
    let tree = match enclosing_git_tree(&path) {
        Ok(None) => return Row::ok(name, format!("{shown}, not in a git tree")),
        Ok(Some(tree)) => tree,
        Err(error) => {
            return Row::issue(
                name,
                format!(
                    "{shown}, git ancestry could not be inspected: {}",
                    error.kind()
                ),
            );
        }
    };
    let detail = format!("{shown}, inside git tree {}", tree.display());
    match key {
        StoreKey::Audio => Row::new(name, Verdict::Warning, detail)
            .advice("confirm the ignore rules exclude audio"),
        StoreKey::Drafts | StoreKey::EngineOutputs => Row::new(name, Verdict::Warning, detail)
            .advice("sensitive: this store holds private JSON; keep it out of the repository"),
        _ => Row::ok(name, detail),
    }
}

fn vault(settings: &Settings) -> Row {
    match resolve_vault(settings) {
        Ok(None) => Row::ok("vault", "not required by the portable profile"),
        Ok(Some(path)) => Row::ok("vault", format!("directory {}", path.display())),
        Err(error) => Row::issue("vault", root_error(&error).message),
    }
}

fn home_link(settings: &Settings) -> Row {
    let home = settings.home.display();
    let Some(target) = &settings.symlink_target else {
        return match inspect_symlink(&settings.home) {
            LinkState::LinkTo(target) => Row::ok(
                "home_link",
                format!("followed, not managed: {home} -> {}", target.display()),
            ),
            LinkState::Unreadable(detail) => Row::issue("home_link", detail),
            _ => Row::ok("home_link", "not configured"),
        };
    };
    match verify_symlink(&settings.home, target) {
        Ok(()) => Row::ok("home_link", format!("{home} -> {}", target.display())),
        Err(SymlinkError::Missing) => {
            Row::issue("home_link", "missing").advice("run `vpt symlink deploy`")
        }
        Err(error) => Row::issue("home_link", format!("{error:?}")),
    }
}
