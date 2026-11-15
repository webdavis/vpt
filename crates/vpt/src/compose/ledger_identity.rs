use super::errors::{ledger_error, open_error, root_error};
use super::{Environment, config_error};
use std::io::Write;
use std::path::{Path, PathBuf};
use vpt_adapters::config::planned_directory;
use vpt_adapters::{MarkerError, Markers, RootDir, SqliteLedger};
use vpt_domain::ledger_identity::{Finding, LedgerIdentity, WriteMark};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) struct Observed {
    pub(crate) folder: PathBuf,
    pub(crate) ledger: Option<LedgerIdentity>,
    pub(crate) marker: Option<WriteMark>,
}

impl Observed {
    pub(crate) fn finding(&self) -> Option<Finding> {
        Finding::of(self.ledger.as_ref(), self.marker.as_ref())
    }
}

/// The markers stay in the default state folder, on this machine, wherever the ledger lives.
pub(crate) fn markers(environment: &Environment) -> Result<Markers, Box<ErrorDocument>> {
    let default = environment.state_dir_default().map_err(config_error)?;
    let folder = match default.strip_prefix("~/") {
        Some(rest) => environment.home_dir().map_err(config_error)?.join(rest),
        None => PathBuf::from(default),
    };
    Ok(Markers::new(folder.join("markers")))
}

pub(crate) fn configured(
    environment: &Environment,
    config: Option<&Path>,
) -> Result<(Observed, Markers), Box<ErrorDocument>> {
    let settings = super::settings_from(environment, config)?;
    let folder = planned_directory(&settings.state_dir, "home.state_dir")
        .map_err(|error| Box::new(root_error(&error)))?;
    let markers = markers(environment)?;
    Ok((observe(&folder, &markers)?, markers))
}

pub(crate) fn observe(folder: &Path, markers: &Markers) -> Result<Observed, Box<ErrorDocument>> {
    let ledger = match SqliteLedger::open_read_only(folder).map_err(|e| Box::new(open_error(e)))? {
        Some(ledger) => ledger.identity().map_err(|e| Box::new(ledger_error(e)))?,
        None => None,
    };
    Ok(Observed {
        folder: folder.to_path_buf(),
        ledger,
        marker: markers.read(folder).map_err(marker_error)?,
    })
}

/// Refuses before anything is written when this machine's last write is not in the ledger, then
/// counts this command as a write and records it in this machine's marker.
pub(crate) fn claim(state: &RootDir, markers: &Markers) -> Result<(), Box<ErrorDocument>> {
    let observed = observe(state.path(), markers)?;
    if let Some(finding) = observed.finding() {
        return Err(Box::new(
            ErrorDocument::new(ErrorKind::Refused, message(&observed.folder, finding))
                .rule(rule(finding)),
        ));
    }
    let ledger = SqliteLedger::open(state).map_err(|error| Box::new(open_error(error)))?;
    let mark = ledger
        .stamp()
        .map_err(|error| Box::new(ledger_error(error)))?;
    drop(ledger);
    markers.write(state.path(), &mark).map_err(marker_error)
}

/// Prints the finding as one line on standard error, so standard output stays as it would be. A
/// failure here is left for the command itself to report.
pub(crate) fn warn(config: Option<&Path>) {
    let environment = Environment::from_process();
    let Ok((observed, _)) = configured(&environment, config) else {
        return;
    };
    if let Some(finding) = observed.finding() {
        let line = format!(
            "vpt: warning: {}: {}",
            rule(finding),
            message(&observed.folder, finding)
        );
        let _ = writeln!(std::io::stderr().lock(), "{line}");
    }
}

pub(crate) fn rule(finding: Finding) -> &'static str {
    match finding {
        Finding::Replaced => "ledger_replaced",
        Finding::Behind => "ledger_behind",
        Finding::Diverged => "ledger_diverged",
    }
}

pub(crate) fn message(folder: &Path, finding: Finding) -> String {
    format!("{}: {}; {ADVICE}", folder.display(), happened(finding))
}

pub(crate) const ADVICE: &str =
    "`vpt ledger show` lists the facts; if this ledger is the one to keep, run: vpt ledger accept";

pub(crate) fn happened(finding: Finding) -> &'static str {
    match finding {
        Finding::Replaced => "this ledger is missing or is not the one this machine last wrote",
        Finding::Behind => "this ledger is older than the one this machine last wrote",
        Finding::Diverged => {
            "another machine wrote this ledger from an older copy, and this machine's last write \
             is gone"
        }
    }
}

pub(crate) fn marker_error(error: MarkerError) -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Store,
        format!(
            "cannot use this machine's ledger marker {}: {}",
            error.path.display(),
            error.detail
        ),
    ))
}
