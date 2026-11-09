use super::{MAX_BYTES, MAX_ITEMS, invalid};
use std::ops::Range;
use vpt_application::ports::LedgerError;

pub(super) fn validate(ranges: &[Range<usize>], remaining: &mut usize) -> Result<(), LedgerError> {
    *remaining = remaining.checked_sub(ranges.len()).ok_or_else(invalid)?;
    let mut previous = None;
    for range in ranges {
        let pair = (range.start, range.end);
        if range.start >= range.end
            || range.end > MAX_ITEMS
            || previous.is_some_and(|prior| prior >= pair)
        {
            return Err(invalid());
        }
        previous = Some(pair);
    }
    Ok(())
}
pub(super) fn encode(ranges: &[Range<usize>], budget: usize) -> Result<String, LedgerError> {
    crate::ledger::json::encode(
        &ranges
            .iter()
            .map(|range| [range.start, range.end])
            .collect::<Vec<_>>(),
        budget,
    )
    .map_err(|_| invalid())
}
pub(in crate::ledger) fn decode(text: &str) -> Result<Vec<Range<usize>>, LedgerError> {
    let document = vpt_protocol::read_bounded(
        text.as_bytes(),
        &vpt_protocol::Limits {
            bytes: MAX_BYTES,
            array_len: MAX_ITEMS,
            ..vpt_protocol::Limits::incoming()
        },
    )
    .map_err(|_| invalid())?;
    let ranges: Vec<[usize; 2]> = serde_json::from_value(document).map_err(|_| invalid())?;
    Ok(ranges.into_iter().map(|[start, end]| start..end).collect())
}
