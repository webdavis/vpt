use super::{Range, Readable, overlaps};

pub(super) fn apply(text: &str, protected: &[Range<usize>], deleted: &[Range<usize>]) -> Readable {
    let mut endpoints: Vec<_> = protected
        .iter()
        .enumerate()
        .flat_map(|(index, range)| [(range.start, index, false), (range.end, index, true)])
        .collect();
    endpoints.sort_unstable();
    let mut next = 0;
    let mut result = Readable {
        text: String::new(),
        protected: vec![0..0; protected.len()],
    };
    let mut initial = true;
    let mut chars = text.char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        track(&mut result, &endpoints, &mut next, offset);
        if overlaps(deleted, offset, offset + ch.len_utf8()) {
            continue;
        }
        if initial && ch.is_alphabetic() && !overlaps(protected, offset, offset + ch.len_utf8()) {
            result.text.extend(ch.to_uppercase());
        } else {
            result.text.push(ch)
        }
        if ch.is_alphanumeric() {
            initial = false
        }
        if matches!(ch, '!' | '?' | '。' | '！' | '？')
            || (ch == '.' && chars.peek().is_none_or(|(_, next)| !next.is_alphanumeric()))
        {
            initial = true
        }
    }
    track(&mut result, &endpoints, &mut next, text.len());
    terminate(&mut result);
    result
}

fn track(
    result: &mut Readable,
    endpoints: &[(usize, usize, bool)],
    next: &mut usize,
    offset: usize,
) {
    while let Some(&(at, index, end)) = endpoints.get(*next) {
        if at > offset {
            break;
        }
        let position = result.text.len();
        if end {
            result.protected[index].end = position
        } else {
            result.protected[index].start = position
        }
        *next += 1;
    }
}

fn terminate(result: &mut Readable) {
    let trimmed = result.text.trim_end();
    if trimmed.is_empty() || crate::transcript::ends_sentence(trimmed) {
        return;
    }
    let mut at = trimmed.len();
    if result
        .protected
        .iter()
        .any(|range| range.start < at && range.end > at)
    {
        at = result.text.len()
    }
    result.text.insert(at, '.');
    for range in &mut result.protected {
        let empty = range.start == range.end;
        if range.start > at || (range.start == at && !empty) {
            range.start += 1
        }
        if range.end > at {
            range.end += 1
        }
    }
}
