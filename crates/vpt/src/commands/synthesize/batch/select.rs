use super::{Failure, Scope};
use crate::{
    commands::synthesize::{errors, observe},
    compose::{RuntimeLedger, errors::ledger_error},
};
use std::io::BufRead;
use vpt_application::ports::ObservationLedger;
use vpt_domain::identity::RecordingId;
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(super) fn candidates(
    scope: &Scope,
    ledger: &RuntimeLedger,
) -> Result<Vec<Result<RecordingId, Failure>>, Box<ErrorDocument>> {
    match scope {
        Scope::All { since } => Ok(ledger
            .observe_recordings(None)
            .map_err(|error| Box::new(ledger_error(error)))?
            .into_iter()
            .map(|observed| observed.record)
            .filter(|record| since.is_none_or(|since| record.captured_at >= since))
            .filter(|record| observe::inputs(ledger, &record.id).is_ok())
            .map(|record| Ok(record.id))
            .collect()),
        Scope::Stdin => Ok(std::io::stdin()
            .lock()
            .lines()
            .map(|line| line.map(|line| line.trim().to_owned()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                ErrorDocument::new(ErrorKind::Usage, "standard input is not readable text")
            })?
            .into_iter()
            .filter(|line| !line.is_empty())
            .map(|line| eligible(line, ledger))
            .collect()),
    }
}

fn eligible(line: String, ledger: &RuntimeLedger) -> Result<RecordingId, Failure> {
    let checked = RecordingId::parse(&line)
        .map_err(|_| errors::usage("invalid recording id"))
        .and_then(|id| observe::inputs(ledger, &id).map(|_| id));
    checked.map_err(|error| Failure { id: line, error })
}
