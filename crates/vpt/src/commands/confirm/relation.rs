pub(super) mod changes;
use crate::compose::relation_target as target;
use vpt_adapters::{Roots, SqliteLedger};
use vpt_application::{Settings, ports::NoteInputsLedger};
use vpt_domain::{identity::RecordingId, relations::Kind};
use vpt_protocol::error::ErrorDocument;

pub(super) struct Request<'a> {
    pub id: RecordingId,
    pub kind: Kind,
    pub target: &'a str,
}

impl<'a> Request<'a> {
    pub(super) fn parse(id: &str, value: &'a str) -> Result<Self, Box<ErrorDocument>> {
        let id = RecordingId::parse(id).map_err(|_| super::usage("invalid recording id"))?;
        if value.len() > 4 * 65_536 || value.chars().count() > 65_536 {
            return Err(super::usage("relation exceeds its text limit"));
        }
        let (kind, target) = value
            .split_once(':')
            .ok_or_else(|| super::usage("relation requires kind:target"))?;
        let kind = Kind::parse(kind).ok_or_else(|| super::usage("unknown relation kind"))?;
        if target.trim().is_empty() || target.chars().any(char::is_control) {
            return Err(super::usage("relation target must contain valid text"));
        }
        Ok(Self { id, kind, target })
    }
}

pub(super) fn preflight(
    request: &Request<'_>,
    settings: &Settings,
    roots: &Roots,
) -> Result<(), Box<ErrorDocument>> {
    let ledger = SqliteLedger::open_read_only(&roots.state_dir)
        .map_err(|error| Box::new(crate::compose::errors::open_error(error)))?
        .ok_or_else(|| super::usage("recording not found"))?;
    let input = ledger
        .note_inputs(&request.id)
        .map_err(|error| Box::new(crate::compose::errors::ledger_error(error)))?
        .ok_or_else(|| super::usage("recording not found"))?;
    let index = crate::compose::index::read(settings, roots)?;
    target::resolve(request.target, request.kind, &input, &ledger, &index).map(|_| ())
}
