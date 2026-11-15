mod view;

use crate::cli::args::ledger::Action;
use crate::cli::output::Outcome;
use crate::compose::Environment;
use crate::compose::ledger_identity::{Observed, configured, marker_error, observe};
use crate::compose::mutation::lock_with_wait;
use std::path::Path;
use std::time::Duration;
use vpt_adapters::RootDir;
use vpt_domain::ledger_identity::LedgerIdentity;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn run(environment: &Environment, config: Option<&Path>, action: &Action) -> Outcome {
    let result = match action {
        Action::Show => configured(environment, config)
            .and_then(|(observed, _)| view::outcome("ledger show", &observed, None)),
        Action::Accept { preview } => accept(environment, config, *preview),
    };
    result.unwrap_or_else(|error| Outcome::Failure(*error))
}

fn accept(
    environment: &Environment,
    config: Option<&Path>,
    preview: bool,
) -> Result<Outcome, Box<ErrorDocument>> {
    let (observed, markers) = configured(environment, config)?;
    refuse_unless_found(&observed)?;
    if preview {
        return view::outcome("ledger accept", &observed, Some(true));
    }
    let _lock = match RootDir::open(&observed.folder) {
        Ok(state) => Some(lock_with_wait(&state, Duration::from_secs(5))?),
        Err(_) => None,
    };
    let observed = observe(&observed.folder, &markers)?;
    refuse_unless_found(&observed)?;
    match observed.ledger.as_ref().and_then(LedgerIdentity::latest) {
        Some(mark) => markers.write(&observed.folder, &mark),
        None => markers.remove(&observed.folder),
    }
    .map_err(marker_error)?;
    let accepted = observe(&observed.folder, &markers)?;
    view::outcome("ledger accept", &accepted, Some(false))
}

fn refuse_unless_found(observed: &Observed) -> Result<(), Box<ErrorDocument>> {
    match observed.finding() {
        Some(_) => Ok(()),
        None => Err(Box::new(ErrorDocument::new(
            ErrorKind::Usage,
            format!(
                "nothing to accept: the ledger in {} holds this machine's last write, or this \
                 machine has not written to it",
                observed.folder.display()
            ),
        ))),
    }
}
