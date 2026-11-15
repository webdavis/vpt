use super::report::{Check, Row};
use crate::compose::{Environment, settings_from};
use std::cell::OnceCell;
use std::path::{Path, PathBuf};
use vpt_adapters::config::{RootError, Roots, resolve};
use vpt_application::Settings;

pub(crate) const NAMES: [&str; 7] = [
    "config",
    "stores",
    "ledger",
    "recordings",
    "helper",
    "engines",
    "leftovers",
];

const SKIPPED: &str = "the configuration did not load; see `vpt doctor config`";

pub(super) struct Context {
    pub(super) settings: Settings,
    pub(super) roots: Result<Roots, RootError>,
    helper: OnceCell<Row>,
}

impl Context {
    pub(super) fn helper(&self) -> &Row {
        self.helper
            .get_or_init(|| super::helper::probe(&self.settings))
    }
}

type Probe = fn(&Context) -> Vec<Row>;

const AFTER_CONFIG: [(&str, &str, Probe); 6] = [
    ("stores", "where vpt keeps its files", super::stores::rows),
    (
        "ledger",
        "vpt's record of recordings, opened read-only",
        super::ledger::rows,
    ),
    (
        "recordings",
        "the Voice Memos folder vpt reads",
        super::source::rows,
    ),
    ("helper", "vpt's own macOS helper", super::helper::rows),
    (
        "engines",
        "each configured engine; available means an executable file, and Apple also needs the \
         helper; no engine is run",
        super::engines::rows,
    ),
    (
        "leftovers",
        "files an interrupted step left behind",
        super::leftovers::rows,
    ),
];

pub(super) fn run(
    environment: &Environment,
    config: Option<&Path>,
    wanted: &[String],
) -> Vec<Check> {
    let wanted = |name: &str| wanted.is_empty() || wanted.iter().any(|named| named == name);
    let loaded = load(environment, config);
    let mut checks = Vec::new();
    if wanted("config") || loaded.is_err() {
        checks.push(Check {
            name: "config",
            blurb: "the configuration file",
            rows: config_rows(&loaded),
            skipped: None,
        });
    }
    let context = loaded.ok().map(|(selected, settings)| {
        let config_dir = selected
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        Context {
            roots: resolve(&settings, config_dir),
            settings,
            helper: OnceCell::new(),
        }
    });
    for (name, blurb, probe) in AFTER_CONFIG.into_iter().filter(|(name, ..)| wanted(name)) {
        checks.push(match &context {
            Some(context) => Check {
                name,
                blurb,
                rows: probe(context),
                skipped: None,
            },
            None => Check {
                name,
                blurb,
                rows: Vec::new(),
                skipped: Some(SKIPPED),
            },
        });
    }
    checks
}

fn load(environment: &Environment, config: Option<&Path>) -> Result<(PathBuf, Settings), String> {
    let selected = config
        .map(Path::to_path_buf)
        .map_or_else(|| environment.config_path(), Ok)
        .map_err(|_| "configuration path is unavailable".to_string())?;
    let settings = settings_from(environment, Some(&selected)).map_err(|error| error.message)?;
    Ok((selected, settings))
}

fn config_rows(loaded: &Result<(PathBuf, Settings), String>) -> Vec<Row> {
    match loaded {
        Ok((selected, settings)) => vec![
            Row::ok("file", selected.display().to_string()),
            Row::ok(
                "version",
                format!("config_version {}", settings.config_version),
            ),
        ],
        Err(message) => vec![Row::issue("file", message).advice("run `vpt setup`")],
    }
}
