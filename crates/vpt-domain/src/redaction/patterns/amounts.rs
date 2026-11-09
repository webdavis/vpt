use super::*;
pub(super) fn scan(
    text: &str,
    enabled: &[Class],
    found: &mut Vec<matches::Match>,
) -> Result<(), Error> {
    let mut chars = text.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if !digit(ch) {
            continue;
        }
        let mut run_start = start;
        let mut end = start + ch.len_utf8();
        let mut digits = 1;
        loop {
            if let Some(&(at, next)) = chars.peek() {
                if digit(next) {
                    chars.next();
                    end = at + next.len_utf8();
                    digits += 1;
                    continue;
                }
                if matches!(next, '.' | ',') && chars.clone().nth(1).is_some_and(|(_, c)| digit(c))
                {
                    number(found, enabled, run_start..end, digits)?;
                    chars.next();
                    let (at, ch) = chars.next().ok_or(Error::InvalidText)?;
                    run_start = at;
                    end = at + ch.len_utf8();
                    digits = 1;
                    continue;
                }
            }
            break;
        }
        number(found, enabled, run_start..end, digits)?;
        let prefix = text[..start].chars().next_back().filter(|c| currency(*c));
        let suffix = text[end..].chars().next().filter(|c| currency(*c));
        if enabled.contains(&Class::Money) && (prefix.is_some() || suffix.is_some()) {
            matches::push(
                found,
                start - prefix.map_or(0, char::len_utf8)..end + suffix.map_or(0, char::len_utf8),
                Class::Money,
            )?;
        }
    }
    Ok(())
}
fn number(
    found: &mut Vec<matches::Match>,
    enabled: &[Class],
    range: std::ops::Range<usize>,
    digits: usize,
) -> Result<(), Error> {
    if digits >= 4 && enabled.contains(&Class::Number) {
        matches::push(found, range, Class::Number)?;
    }
    Ok(())
}
