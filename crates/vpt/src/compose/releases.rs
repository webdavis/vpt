use super::{errors::ledger_error, history::commit, publication::merge_completed};
use std::collections::BTreeSet;
use vpt_adapters::{ReleaseFiles, Roots, SystemClock};
use vpt_application::ports::{
    Clock, EventSink, ReleaseHistory, ReleaseLedger, RunHistory, SinkWarning,
};
use vpt_domain::run::RunKind;
use vpt_protocol::error::{ErrorDocument, ErrorKind};
pub(crate) mod errors;
pub(crate) use super::history::finish;

pub(crate) fn repair(
    ledger: &(impl ReleaseLedger + ReleaseHistory + RunHistory),
    roots: &Roots,
    sink: &mut impl EventSink,
    warnings: &mut BTreeSet<SinkWarning>,
) -> Result<Vec<String>, Box<ErrorDocument>> {
    let entries = ledger
        .pending_releases()
        .map_err(|e| Box::new(ledger_error(e)))?;
    if entries.is_empty() {
        return Ok(vec![]);
    }
    let run = commit(
        ledger.begin_run(RunKind::Publish, SystemClock.now()),
        sink,
        warnings,
    )?
    .run;
    let mut completed = Vec::new();
    let result = (|| {
        for sequence in entries {
            let payload = ledger
                .release_payload(sequence)
                .map_err(|e| Box::new(ledger_error(e)))?
                .ok_or_else(|| {
                    Box::new(ErrorDocument::new(
                        ErrorKind::Ledger,
                        "pending release has no saved payload",
                    ))
                })?;
            let destination = payload.release.destination.parent().ok_or_else(|| {
                Box::new(ErrorDocument::new(
                    ErrorKind::Ledger,
                    "pending release has invalid destination",
                ))
            })?;
            let files = ReleaseFiles::open(roots, destination).map_err(errors::document)?;
            let verified = files.publish(&payload).map_err(errors::document)?;
            let receipt = commit(
                ledger.complete_release(run, sequence, verified, SystemClock.now()),
                sink,
                warnings,
            )?;
            for id in receipt.completed_ids() {
                if !completed.contains(&id) {
                    completed.push(id);
                }
            }
        }
        Ok::<_, Box<ErrorDocument>>(())
    })();
    let result = finish(ledger, run, result, sink, warnings);
    match result {
        Ok(()) => Ok(completed),
        Err(mut error) => {
            merge_completed(&mut error, completed);
            Err(error)
        }
    }
}
#[cfg(test)]
mod tests;
