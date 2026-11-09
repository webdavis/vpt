use super::invalid;
use vpt_application::ports::LedgerError;
use vpt_domain::brief::document::Snapshot;
mod decode;
mod encode;
mod model;
pub(crate) const MAX_BYTES: usize = 32 * 1024 * 1024;

pub(crate) fn encode(snapshot: &Snapshot) -> Result<String, LedgerError> {
    snapshot.document().validate().map_err(|_| invalid())?;
    crate::ledger::json::encode(&encode::value(snapshot), MAX_BYTES).map_err(|_| invalid())
}
pub(crate) fn decode(text: &str) -> Result<Snapshot, LedgerError> {
    let value = vpt_protocol::read_bounded(
        text.as_bytes(),
        &vpt_protocol::Limits {
            bytes: MAX_BYTES,
            depth: 8,
            text_chars: 65_536,
            array_len: 65_536,
        },
    )
    .map_err(|_| invalid())?;
    decode::snapshot(serde_json::from_value(value).map_err(|_| invalid())?)
}

#[cfg(test)]
mod tests;
