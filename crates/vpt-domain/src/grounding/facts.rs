use super::{GroundingError, MAX_ITEMS, MAX_TEXT_BYTES, spend};
use caseless::Caseless;
use unicode_normalization::UnicodeNormalization;

mod money;
mod phrases;
mod tokens;

pub(super) struct Fact {
    pub text: String,
    pub key: String,
    pub required: bool,
}

pub(super) fn extract(text: &str, remaining: &mut usize) -> Result<Vec<Fact>, GroundingError> {
    spend(remaining, text.len())?;
    let normalized = collect(text.nfkc(), remaining)?;
    let tokens = tokens::scan(&normalized)?;
    let mut facts = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        for phrase in phrases::at(&normalized, &tokens, index) {
            add(&mut facts, &phrase, true, remaining)?;
        }
        if let Some(phrase) = money::at(&normalized, &tokens, index, remaining)? {
            add(&mut facts, &phrase, true, remaining)?;
        }
        let text = token.text(&normalized);
        let numeric = text.chars().any(|c| c.is_numeric());
        let capitalized = text
            .chars()
            .find(|c| c.is_alphabetic())
            .is_some_and(char::is_uppercase);
        let numeric_expression = numeric && !text.chars().any(|c| c.is_alphabetic());
        add(
            &mut facts,
            text,
            numeric_expression || capitalized,
            remaining,
        )?;
        for digits in text
            .split(|c: char| !c.is_numeric())
            .filter(|s| !s.is_empty())
        {
            if digits != text {
                add(&mut facts, digits, true, remaining)?;
            }
        }
    }
    Ok(facts)
}

fn add(
    facts: &mut Vec<Fact>,
    text: &str,
    required: bool,
    remaining: &mut usize,
) -> Result<(), GroundingError> {
    if facts.len() == MAX_ITEMS {
        return Err(GroundingError::CapacityExceeded);
    }
    let key = collect(text.nfkc().default_case_fold().nfkc(), remaining)?;
    let key = if text.chars().any(tokens::currency) {
        key.chars().filter(|c| !c.is_whitespace()).collect()
    } else {
        key
    };
    spend(remaining, text.len() + 1)?;
    facts.push(Fact {
        text: text.into(),
        key,
        required,
    });
    Ok(())
}

fn collect(
    chars: impl Iterator<Item = char>,
    remaining: &mut usize,
) -> Result<String, GroundingError> {
    let mut result = String::new();
    for ch in chars {
        if result.len() + ch.len_utf8() > MAX_TEXT_BYTES {
            return Err(GroundingError::CapacityExceeded);
        }
        spend(remaining, ch.len_utf8())?;
        result.push(ch);
    }
    Ok(result)
}
