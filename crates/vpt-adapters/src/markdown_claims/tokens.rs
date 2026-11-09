use super::{ClaimError, MAX_ITEMS, blocks::Block};
use std::ops::Range;

pub(super) struct Token {
    pub range: Range<usize>,
    pub raw: Range<usize>,
    pub annotation: bool,
    pub correction: bool,
    pub reference: bool,
}

pub(super) fn scan(
    block: &Block,
    source: &str,
    strip_corrections: bool,
) -> Result<Vec<Token>, ClaimError> {
    let mut tokens = Vec::new();
    let mut open = None;
    for (at, ch) in block.text.char_indices() {
        if !matches!(ch, '[' | ']') || !block.syntax(at) || escaped(source, block.start(at)) {
            continue;
        }
        if ch == '[' {
            open = Some(at);
            continue;
        }
        let Some(start) = open.take() else {
            continue;
        };
        let range = start..at + 1;
        let mut raw = block.raw(range.clone());
        let value = &block.text[start + 1..at];
        let original = &source[raw.clone()];
        let correction = original.starts_with("[corrected: ")
            && original.ends_with(']')
            && !original.contains('\n');
        let marked = correction
            && source[raw.end..].starts_with(vpt_domain::note::FINDING_CORRECTION_MARKER);
        let annotation = (correction && (strip_corrections || marked))
            || matches!(
                original,
                "[unsourced]" | "[bad-reference]" | "[built-on-flagged-text]"
            )
            || (original.starts_with("[unsupported: ")
                && original.ends_with(']')
                && !original.contains('\n'));
        let reference = !correction
            && (value.trim().is_empty()
                || value.trim() == "-"
                || (value.contains(':') && value.contains('-')));
        if annotation || reference || correction {
            if marked {
                raw.end += vpt_domain::note::FINDING_CORRECTION_MARKER.len();
            }
            if tokens.len() >= MAX_ITEMS {
                return Err(ClaimError::CapacityExceeded);
            }
            tokens.push(Token {
                range,
                raw,
                annotation,
                correction,
                reference,
            });
        }
    }
    Ok(tokens)
}

fn escaped(source: &str, offset: usize) -> bool {
    source.as_bytes()[..offset]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        != 0
}
