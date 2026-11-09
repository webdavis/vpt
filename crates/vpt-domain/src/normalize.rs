use caseless::Caseless;
use unicode_normalization::UnicodeNormalization;
mod whole_terms;
pub use whole_terms::{MatchError, TermMatcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub original_index: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentTokens {
    pub tokens: Vec<Token>,
    pub dropped_indices: Vec<usize>,
}

pub fn canonical(text: &str) -> String {
    text.nfkc().default_case_fold().nfkc().collect()
}

pub fn alignment(words: &[&str]) -> AlignmentTokens {
    let mut result = AlignmentTokens {
        tokens: Vec::new(),
        dropped_indices: Vec::new(),
    };
    for (original_index, word) in words.iter().enumerate() {
        let mut text: String = canonical(word)
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .collect();
        for suffix in ["st", "nd", "rd", "th"] {
            if let Some(digits) = text.strip_suffix(suffix)
                && !digits.is_empty()
                && digits.bytes().all(|byte| byte.is_ascii_digit())
            {
                text.truncate(digits.len());
                break;
            }
        }
        if text.is_empty() {
            result.dropped_indices.push(original_index);
        } else {
            result.tokens.push(Token {
                original_index,
                text,
            });
        }
    }
    result
}

pub fn whole_term_matches(words: &[&str], term: &str) -> Vec<std::ops::Range<usize>> {
    let term: Vec<_> = canonical(term)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    if term.is_empty() || term.len() > words.len() {
        return Vec::new();
    }
    let words: Vec<_> = words.iter().map(|word| canonical(word)).collect();
    words
        .windows(term.len())
        .enumerate()
        .filter(|(_, window)| *window == term)
        .map(|(start, _)| start..start + term.len())
        .collect()
}

#[cfg(test)]
mod tests;
