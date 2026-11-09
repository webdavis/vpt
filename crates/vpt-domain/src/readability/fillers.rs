use super::{Range, overlaps};
use crate::normalize;

pub(super) fn deletions(text: &str, protected: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut result: Vec<Range<usize>> = Vec::new();
    let mut cursor = 0;
    for word in text.split_whitespace() {
        let start = cursor + text[cursor..].find(word).unwrap_or(0);
        let end = start + word.len();
        cursor = end;
        let core = word.trim_matches(|ch: char| !ch.is_alphanumeric());
        let canonical = normalize::canonical(core);
        let after_core = start + word.find(core).unwrap_or(0) + core.len();
        let comma = after_core + text[after_core..].len() - text[after_core..].trim_start().len();
        let like = canonical == "like" && text[comma..].starts_with(',');
        if (!matches!(canonical.as_str(), "um" | "uh" | "erm") && !like)
            || overlaps(protected, start, end)
        {
            continue;
        }
        let mut range = start..end;
        if like && comma >= end && !overlaps(protected, end, comma + 1) {
            range.end = comma + 1
        }
        let after = range.end + text[range.end..].len() - text[range.end..].trim_start().len();
        if after > range.end && !overlaps(protected, range.end, after) {
            range.end = after;
        } else if range.end == text.len() {
            let before = text[..start].trim_end().len();
            if !overlaps(protected, before, start) {
                range.start = before
            }
        }
        if let Some(previous) = result.last_mut()
            && previous.end >= range.start
        {
            previous.end = previous.end.max(range.end);
        } else {
            result.push(range)
        }
    }
    result
}
