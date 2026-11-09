use super::tokens::{Token, currency};

pub(super) fn at(text: &str, tokens: &[Token], index: usize) -> Vec<String> {
    let Some(pair) = tokens.get(index..index + 2) else {
        return Vec::new();
    };
    if !between(text, &pair[0], &pair[1], false) {
        return Vec::new();
    }
    let first = pair[0].text(text);
    let second = pair[1].text(text);
    let date = (month(first) && day(second)) || (day(first) && month(second));
    let money = (currency_code(first) && amount(second))
        || (amount(first)
            && (currency_code(second)
                || currency_name(second)
                || (second.chars().count() == 1 && second.chars().all(currency))));
    if !date && !money {
        return Vec::new();
    }
    let mut phrases = vec![format!("{first} {second}")];
    if date
        && let Some(year) = tokens.get(index + 2)
        && between(text, &pair[1], year, true)
        && year.text(text).len() == 4
        && year.text(text).bytes().all(|b| b.is_ascii_digit())
    {
        phrases.push(format!("{first} {second} {}", year.text(text)));
    }
    phrases
}

fn between(text: &str, first: &Token, second: &Token, comma: bool) -> bool {
    text[first.end..second.start]
        .chars()
        .all(|ch| ch.is_whitespace() || (comma && ch == ','))
}

fn month(text: &str) -> bool {
    if text.len() > 9 {
        return false;
    }
    matches!(
        text.to_ascii_lowercase().as_str(),
        "jan"
            | "january"
            | "feb"
            | "february"
            | "mar"
            | "march"
            | "apr"
            | "april"
            | "may"
            | "jun"
            | "june"
            | "jul"
            | "july"
            | "aug"
            | "august"
            | "sep"
            | "sept"
            | "september"
            | "oct"
            | "october"
            | "nov"
            | "november"
            | "dec"
            | "december"
    )
}

fn day(text: &str) -> bool {
    let text = ["st", "nd", "rd", "th"]
        .into_iter()
        .find_map(|s| text.strip_suffix(s))
        .unwrap_or(text);
    text.parse::<u32>()
        .is_ok_and(|value| (1..=31).contains(&value))
}

pub(super) fn amount(text: &str) -> bool {
    text.chars().any(|ch| ch.is_numeric())
        && text
            .chars()
            .all(|ch| ch.is_numeric() || matches!(ch, '.' | ',' | '+' | '-'))
}

pub(super) fn currency_code(text: &str) -> bool {
    if text.len() != 3 {
        return false;
    }
    matches!(
        text.to_ascii_uppercase().as_str(),
        "USD"
            | "EUR"
            | "GBP"
            | "JPY"
            | "CNY"
            | "CAD"
            | "AUD"
            | "NZD"
            | "CHF"
            | "INR"
            | "KRW"
            | "HKD"
            | "SGD"
            | "MXN"
            | "BRL"
            | "ZAR"
            | "SEK"
            | "NOK"
            | "DKK"
    )
}

pub(super) fn currency_name(text: &str) -> bool {
    if text.len() > 7 {
        return false;
    }
    matches!(
        text.to_ascii_lowercase().as_str(),
        "dollar"
            | "dollars"
            | "cent"
            | "cents"
            | "euro"
            | "euros"
            | "pound"
            | "pounds"
            | "yen"
            | "yuan"
            | "rupee"
            | "rupees"
            | "franc"
            | "francs"
            | "peso"
            | "pesos"
            | "won"
    )
}
