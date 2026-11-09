use super::*;
use std::ops::Range;
pub struct MaskedText {
    pub text: String,
    pub masks: Vec<Range<usize>>,
}
#[derive(Clone, PartialEq, Eq)]
pub struct Candidate {
    pub token: String,
    pub bytes: Range<usize>,
}
impl std::fmt::Debug for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Candidate")
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}
pub fn candidates(text: &str, masks: &[Range<usize>]) -> Result<Vec<Candidate>, Error> {
    validate_text(text)?;
    validate_ranges(text, masks)?;
    let mut output = Vec::new();
    let mut initial = true;
    let mut mask = 0;
    for token in vocabulary::tokens(text)? {
        while masks.get(mask).is_some_and(|r| r.end <= token.range.start) {
            mask += 1;
        }
        if masks.get(mask).is_some_and(|r| r.start < token.range.end) {
            initial = false;
            continue;
        }
        let value = &text[token.range.clone()];
        let first = value.chars().next().ok_or(Error::InvalidText)?;
        if matches!(first, '.' | '!' | '?' | '。' | '！' | '？') {
            initial = true;
        } else if first.is_alphanumeric() {
            let uppercase = first.is_uppercase()
                || unicode_general_category::get_general_category(first)
                    == unicode_general_category::GeneralCategory::TitlecaseLetter;
            if !initial && uppercase {
                output.push(Candidate {
                    token: value.into(),
                    bytes: token.range,
                });
            }
            initial = false;
        }
    }
    Ok(output)
}
pub(super) fn validate_ranges(text: &str, ranges: &[Range<usize>]) -> Result<(), Error> {
    if ranges.len() > MAX_ITEMS {
        return Err(Error::Capacity);
    }
    let mut end = 0;
    for range in ranges {
        if range.start < end
            || range.start >= range.end
            || range.end > text.len()
            || !text.is_char_boundary(range.start)
            || !text.is_char_boundary(range.end)
        {
            return Err(Error::InvalidText);
        }
        end = range.end;
    }
    Ok(())
}
#[cfg(test)]
mod tests;
