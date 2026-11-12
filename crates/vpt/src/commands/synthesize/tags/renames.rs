use crate::compose::errors::ledger_error;
use vpt_adapters::SqliteLedger;
use vpt_application::ports::{TagLedger, TagRename};
use vpt_domain::tags::State;
use vpt_protocol::error::ErrorDocument;

pub(super) type Saved = (Vec<TagRename>, Vec<(String, State)>);

pub(super) fn read(ledger: &SqliteLedger) -> Result<Saved, Box<ErrorDocument>> {
    let error = |error| Box::new(ledger_error(error));
    Ok((
        ledger.tag_renames().map_err(error)?,
        ledger.tag_states().map_err(error)?,
    ))
}
