use super::corrupt;
use vpt_application::ports::LedgerError;
use vpt_domain::{
    grounding::Finding,
    review::{FlagContent, Resolution, ReviewFlag},
};

pub(in crate::ledger) fn check<'a>(
    flags: impl Iterator<Item = &'a ReviewFlag>,
) -> Result<(), LedgerError> {
    let mut bytes = 0usize;
    for (index, flag) in flags.enumerate() {
        // This includes an upper bound for serialized keys, digests, class names and separators.
        bytes = bytes.saturating_add(512);
        let (text, alternative, ranges) = match &flag.content {
            FlagContent::Lexical(value) => (
                value.record_text.len(),
                value.alternative_text.as_ref().map_or(0, String::len),
                value.ranges.len(),
            ),
            FlagContent::Diagnostic(_) => (0, 0, 0),
            FlagContent::Verification(value) => (
                value.record_text.len(),
                match &value.finding {
                    Finding::Unsupported(token) => token.len(),
                    _ => 0,
                },
                value.sources.len(),
            ),
        };
        bytes = bytes
            .saturating_add(text)
            .saturating_add(alternative)
            .saturating_add(ranges.saturating_mul(80));
        if let Resolution::Corrected { text, .. } = &flag.resolution {
            bytes = bytes.saturating_add(text.len());
        }
        if index >= 65_539 || bytes > 48 * 1024 * 1024 {
            return Err(corrupt());
        }
    }
    Ok(())
}
