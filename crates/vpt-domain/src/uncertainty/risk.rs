use super::stream::Stream;
use crate::normalize;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Risk {
    Name,
    Quantity,
}

pub(super) fn kind(stream: &Stream<'_>, index: usize, english: bool) -> Option<Risk> {
    let word = stream.text[index];
    let quantity = word.chars().any(char::is_numeric) || (english && english_quantity(word));
    if quantity {
        Some(Risk::Quantity)
    } else if stream.proper(index) {
        Some(Risk::Name)
    } else {
        None
    }
}

fn english_quantity(word: &str) -> bool {
    normalize::canonical(word)
        .trim_matches(|ch: char| !ch.is_alphanumeric())
        .split('-')
        .all(|part| {
            matches!(
                part,
                "zero"
                    | "one"
                    | "two"
                    | "three"
                    | "four"
                    | "five"
                    | "six"
                    | "seven"
                    | "eight"
                    | "nine"
                    | "ten"
                    | "eleven"
                    | "twelve"
                    | "thirteen"
                    | "fourteen"
                    | "fifteen"
                    | "sixteen"
                    | "seventeen"
                    | "eighteen"
                    | "nineteen"
                    | "twenty"
                    | "thirty"
                    | "forty"
                    | "fifty"
                    | "sixty"
                    | "seventy"
                    | "eighty"
                    | "ninety"
                    | "hundred"
                    | "thousand"
                    | "million"
                    | "billion"
                    | "trillion"
                    | "first"
                    | "second"
                    | "third"
                    | "fourth"
                    | "fifth"
                    | "sixth"
                    | "seventh"
                    | "eighth"
                    | "ninth"
                    | "tenth"
                    | "eleventh"
                    | "twelfth"
                    | "thirteenth"
                    | "fourteenth"
                    | "fifteenth"
                    | "sixteenth"
                    | "seventeenth"
                    | "eighteenth"
                    | "nineteenth"
                    | "twentieth"
                    | "thirtieth"
                    | "fortieth"
                    | "fiftieth"
                    | "sixtieth"
                    | "seventieth"
                    | "eightieth"
                    | "ninetieth"
                    | "hundredth"
                    | "thousandth"
                    | "millionth"
                    | "billionth"
                    | "trillionth"
                    | "january"
                    | "february"
                    | "march"
                    | "april"
                    | "may"
                    | "june"
                    | "july"
                    | "august"
                    | "september"
                    | "october"
                    | "november"
                    | "december"
                    | "monday"
                    | "tuesday"
                    | "wednesday"
                    | "thursday"
                    | "friday"
                    | "saturday"
                    | "sunday"
                    | "noon"
                    | "midnight"
                    | "am"
                    | "pm"
                    | "a.m"
                    | "p.m"
                    | "o'clock"
                    | "dollar"
                    | "dollars"
                    | "cent"
                    | "cents"
                    | "euro"
                    | "euros"
                    | "pound"
                    | "pounds"
                    | "yen"
                    | "yuan"
                    | "usd"
                    | "eur"
                    | "gbp"
            )
        })
}
