use super::*;
mod amounts;
use unicode_general_category::{GeneralCategory, get_general_category};
fn digit(ch: char) -> bool {
    get_general_category(ch) == GeneralCategory::DecimalNumber
}
fn currency(ch: char) -> bool {
    get_general_category(ch) == GeneralCategory::CurrencySymbol
}
pub(super) fn scan(
    text: &str,
    enabled: &[Class],
    found: &mut Vec<matches::Match>,
) -> Result<(), Error> {
    let mut start = 0;
    for token in text.split_inclusive(char::is_whitespace) {
        let value = token.trim_end_matches(char::is_whitespace);
        let range = start..start + value.len();
        if enabled.contains(&Class::Email) && value.contains('@') {
            matches::push(found, range.clone(), Class::Email)?;
        }
        if enabled.contains(&Class::Url) && url(value) {
            matches::push(found, range, Class::Url)?;
        }
        start += token.len();
    }
    amounts::scan(text, enabled, found)?;
    if enabled.contains(&Class::Phone) {
        phones(text, found)?;
    }
    Ok(())
}
fn url(token: &str) -> bool {
    if token
        .get(..4)
        .is_some_and(|s| s.eq_ignore_ascii_case("www."))
    {
        return true;
    }
    token.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme.as_bytes()[0].is_ascii_alphabetic()
            && scheme
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"+.-".contains(&b))
    })
}
fn phones(text: &str, found: &mut Vec<matches::Match>) -> Result<(), Error> {
    let (mut start, mut digits) = (0, 0);
    for (end, ch) in text
        .char_indices()
        .chain(std::iter::once((text.len(), '\0')))
    {
        if digit(ch) {
            digits += 1;
            continue;
        }
        if matches!(ch, ' ' | '(' | ')' | '+' | '.' | '-') {
            continue;
        }
        if digits >= 7 {
            let run = &text[start..end];
            let trimmed = run.trim_matches([' ', '.', '-']);
            let first = start + run.len() - run.trim_start_matches([' ', '.', '-']).len();
            if !trimmed.is_empty() {
                matches::push(found, first..first + trimmed.len(), Class::Phone)?;
            }
        }
        start = end + ch.len_utf8();
        digits = 0;
    }
    Ok(())
}
