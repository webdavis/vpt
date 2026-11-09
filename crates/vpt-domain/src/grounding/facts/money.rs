use super::{
    GroundingError,
    phrases::{amount, currency_code, currency_name},
    spend,
    tokens::{Token, currency},
};

pub(super) fn at(
    text: &str,
    tokens: &[Token],
    index: usize,
    remaining: &mut usize,
) -> Result<Option<String>, GroundingError> {
    let first = tokens[index].text(text);
    let prefix = currency_code(first) || symbol(first);
    if !prefix && (!number(first) || continues(text, tokens, index)) {
        return Ok(None);
    }
    let mut phrase = String::new();
    let mut spelled = false;
    let mut position = index;
    if prefix {
        append(&mut phrase, first, remaining)?;
        position += 1;
    }
    while let Some(token) = tokens.get(position) {
        if position > index && !adjacent(text, &tokens[position - 1], token) {
            break;
        }
        let part = token.text(text);
        if number(part) {
            spelled |= word(part);
            append(&mut phrase, part, remaining)?;
        } else if part.eq_ignore_ascii_case("and")
            && position > index
            && tokens
                .get(position + 1)
                .is_some_and(|next| adjacent(text, token, next) && number(next.text(text)))
        {
            append(&mut phrase, part, remaining)?;
        } else if spelled && (currency_code(part) || currency_name(part) || symbol(part)) {
            append(&mut phrase, part, remaining)?;
            return Ok(Some(phrase));
        } else {
            break;
        }
        position += 1;
    }
    Ok((prefix && spelled).then_some(phrase))
}

fn continues(text: &str, tokens: &[Token], index: usize) -> bool {
    let Some(previous) = index.checked_sub(1).and_then(|i| tokens.get(i)) else {
        return false;
    };
    if !adjacent(text, previous, &tokens[index]) {
        return false;
    }
    number(previous.text(text))
        || (previous.text(text).eq_ignore_ascii_case("and")
            && index > 1
            && adjacent(text, &tokens[index - 2], previous)
            && number(tokens[index - 2].text(text)))
}

fn adjacent(text: &str, first: &Token, second: &Token) -> bool {
    text[first.end..second.start]
        .chars()
        .all(char::is_whitespace)
}

fn symbol(text: &str) -> bool {
    text.chars().count() == 1 && text.chars().all(currency)
}

fn number(text: &str) -> bool {
    amount(text) || word(text)
}

fn word(text: &str) -> bool {
    text.len() <= 32
        && text.to_ascii_lowercase().split('-').all(|part| {
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
                    | "quadrillion"
            )
        })
}

fn append(output: &mut String, text: &str, remaining: &mut usize) -> Result<(), GroundingError> {
    spend(remaining, text.len() + 1)?;
    if !output.is_empty() {
        output.push(' ');
    }
    output.push_str(text);
    Ok(())
}
