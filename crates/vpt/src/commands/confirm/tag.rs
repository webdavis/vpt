pub(super) mod changes;

use vpt_adapters::{Roots, SqliteLedger};
use vpt_application::ports::RecordingLedger;
use vpt_domain::{
    identity::RecordingId,
    tags::{Action, Vocabulary},
    time::UtcInstant,
};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Request<'a> {
    pub id: RecordingId,
    pub value: &'a str,
    pub action: Action,
}

pub(super) fn validate(request: &Request<'_>) -> Result<(), Box<ErrorDocument>> {
    Vocabulary::new(&[])
        .expect("empty vocabulary")
        .edit(
            &[],
            request.value,
            request.action,
            u32::MAX,
            UtcInstant { secs: 0 },
        )
        .map(|_| ())
        .map_err(|_| super::usage("tag must contain bounded, valid tag text"))
}

pub(super) fn preflight(request: &Request<'_>, roots: &Roots) -> Result<(), Box<ErrorDocument>> {
    let ledger = SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|error| Box::new(crate::compose::errors::open_error(error)))?
        .ok_or_else(|| super::usage("recording not found"))?;
    ledger
        .by_id(&request.id)
        .map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))?
        .ok_or_else(|| super::usage("recording not found"))?;
    Ok(())
}
