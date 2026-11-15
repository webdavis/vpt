use super::config;
use crate::compose::Environment;
use crate::report::{Check, Row};
use std::cell::OnceCell;
use std::path::Path;
use vpt_adapters::Markers;
use vpt_adapters::config::{RootError, Roots, resolve};
use vpt_application::Settings;

pub(crate) const NAMES: [&str; 11] = [
    "config",
    "stores",
    "ledger",
    "recordings",
    "helper",
    "engines",
    "speakers",
    "summary",
    "actions",
    "calendar",
    "leftovers",
];

const SKIPPED: &str = "the configuration did not load; see `vpt doctor config`";

pub(super) struct Context {
    pub(super) settings: Settings,
    pub(super) roots: Result<Roots, RootError>,
    pub(super) markers: Option<Markers>,
    helper: OnceCell<Row>,
}

impl Context {
    pub(super) fn helper(&self) -> &Row {
        self.helper
            .get_or_init(|| super::helper::probe(&self.settings))
    }
}

pub(super) fn named(wanted: &[String], name: &str) -> bool {
    wanted.is_empty() || wanted.iter().any(|named| named == name)
}

type Probe = fn(&Context) -> Vec<Row>;

const AFTER_CONFIG: [(&str, &str, Probe); 10] = [
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
        "speakers",
        "speaker labels, which need a Hugging Face token on a selected whisply engine",
        super::speakers::rows,
    ),
    (
        "summary",
        "the analysis command that writes each summary; no command is run",
        super::summary::rows,
    ),
    (
        "actions",
        "the commands that send and withdraw follow-ups; no command is run",
        super::actions::rows,
    ),
    (
        "calendar",
        "where vpt reads meetings from; nothing is opened or sent",
        super::calendar::rows,
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
    let wanted = |name: &str| named(wanted, name);
    let loaded = config::load(environment, config);
    let mut checks = Vec::new();
    if wanted("config") || loaded.is_err() {
        checks.push(Check {
            name: "config",
            blurb: "the configuration file",
            rows: config::rows(environment, config, &loaded),
            skipped: None,
        });
    }
    let context = loaded.ok().map(|(selected, settings)| Context {
        roots: resolve(&settings, config::directory(&selected)),
        markers: crate::compose::ledger_identity::markers(environment).ok(),
        settings,
        helper: OnceCell::new(),
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
