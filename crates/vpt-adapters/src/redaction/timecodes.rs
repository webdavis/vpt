use super::*;
pub(super) fn prepare(
    text: &str,
    protected: &[Range<usize>],
    keep: bool,
) -> Result<(String, Vec<Range<usize>>), Error> {
    if keep {
        return Ok((text.into(), protected.to_vec()));
    }
    let mut output = String::with_capacity(text.len());
    let mut removed = Vec::new();
    let mut cursor = 0;
    let mut total = 0;
    while let Some(start) = text[cursor..].find('[').map(|at| at + cursor) {
        output.push_str(&text[cursor..start]);
        let Some(end) = text[start..].find(']').map(|at| at + start) else {
            cursor = start;
            break;
        };
        let parts: Vec<_> = text[start + 1..end].split('-').take(3).collect();
        if parts.is_empty() || parts.len() > 2 || !parts.iter().all(|part| valid(part)) {
            output.push_str(&text[start..=end]);
        } else {
            if removed.len() >= MAX_ITEMS {
                return Err(Error::Capacity);
            }
            total += end + 1 - start;
            removed.push((end + 1, total));
        }
        cursor = end + 1;
    }
    output.push_str(&text[cursor..]);
    let at = |offset| {
        let index = removed.partition_point(|(end, _)| *end <= offset);
        offset - if index == 0 { 0 } else { removed[index - 1].1 }
    };
    let protected = protected.iter().map(|r| at(r.start)..at(r.end)).collect();
    Ok((output, protected))
}
fn valid(text: &str) -> bool {
    let parts: Vec<_> = text.split(':').take(4).collect();
    (2..=3).contains(&parts.len())
        && !parts[0].is_empty()
        && parts[0].bytes().all(|b| b.is_ascii_digit())
        && parts[1..].iter().all(|part| {
            part.len() == 2
                && part.bytes().all(|b| b.is_ascii_digit())
                && part.parse::<u8>().is_ok_and(|v| v < 60)
        })
}
