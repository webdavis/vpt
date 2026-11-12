use super::{errors, output, runtime};
use crate::compose::RuntimeLedger;
use vpt_adapters::{ClonefileArchive, ContainedError, HelperClient, Roots, VoiceMemosStore};
use vpt_application::{Mode, SourceSettings};
use vpt_protocol::error::ErrorDocument;

pub(super) fn run(
    recorder: Result<VoiceMemosStore, ContainedError>,
    roots: &Roots,
    helper: &HelperClient,
    source: &SourceSettings,
    mode: &Mode,
) -> Result<crate::cli::output::Outcome, Box<ErrorDocument>> {
    let recorder = recorder.map_err(|error| Box::new(errors::source(error)))?;
    let ledger = RuntimeLedger::read_only(&roots.state_dir)?;
    let archive = ClonefileArchive::open_read_only(&roots.stores.audio)
        .map_err(|error| Box::new(crate::compose::errors::contained_error(error)))?;
    let mut sink = crate::compose::events::Observation;
    let report = match &ledger {
        RuntimeLedger::Sqlite(ledger) => {
            runtime::execute(&recorder, &archive, ledger, helper, &mut sink, source, mode)
        }
        RuntimeLedger::Empty(ledger) => {
            runtime::execute(&recorder, &archive, ledger, helper, &mut sink, source, mode)
        }
    };
    Ok(output::outcome(report, mode))
}
