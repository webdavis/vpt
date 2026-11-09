use super::*;
use crate::normalize::{MatchError, TermMatcher};
use std::ops::Range;
use unicode_normalization::char::is_combining_mark;
pub(super) struct Token {
    pub(super) range: Range<usize>,
    text: String,
}
pub(super) struct Vocabulary {
    matcher: TermMatcher,
    class: Class,
}
impl Vocabulary {
    pub fn new(values: &[&str], class: Class) -> Result<Self, Error> {
        if values.len() > MAX_ITEMS
            || values
                .iter()
                .try_fold(0usize, |n, v| n.checked_add(v.len()))
                .is_none_or(|n| n > MAX_TEXT)
        {
            return Err(Error::Capacity);
        }
        let mut texts = Vec::new();
        let mut bytes = 0;
        for value in values {
            validate_text(value)?;
            let text = tokens(value)?
                .into_iter()
                .map(|t| t.text)
                .collect::<Vec<_>>()
                .join(" ");
            bytes += text.len();
            if bytes > MAX_TEXT {
                return Err(Error::Capacity);
            }
            texts.push(text);
        }
        let refs: Vec<_> = texts.iter().map(String::as_str).collect();
        Ok(Self {
            matcher: TermMatcher::new(&refs).map_err(|_| Error::Capacity)?,
            class,
        })
    }
    pub fn scan(&self, tokens: &[Token], found: &mut Vec<matches::Match>) -> Result<(), Error> {
        let refs: Vec<_> = tokens.iter().map(|t| t.text.as_str()).collect();
        self.matcher
            .scan(&refs, |_, range| {
                matches::push(
                    found,
                    tokens[range.start].range.start..tokens[range.end - 1].range.end,
                    self.class,
                )
                .map_err(|_| MatchError::CapacityExceeded)
            })
            .map_err(|_| Error::Capacity)
    }
    pub fn first(&self, tokens: &[Token]) -> Option<Result<usize, Error>> {
        let refs: Vec<_> = tokens.iter().map(|t| t.text.as_str()).collect();
        let mut first = None;
        if self
            .matcher
            .scan(&refs, |index, _| {
                first.get_or_insert(index);
                Ok(())
            })
            .is_err()
        {
            return Some(Err(Error::Capacity));
        }
        first.map(Ok)
    }
}
pub(super) fn tokens(text: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        let mut end = start + ch.len_utf8();
        if ch.is_alphanumeric() || ch == '_' || is_combining_mark(ch) {
            while let Some(&(at, next)) = chars.peek() {
                let word = next.is_alphanumeric() || next == '_' || is_combining_mark(next);
                let apostrophe = matches!(next, '\'' | '’')
                    && chars
                        .clone()
                        .nth(1)
                        .is_some_and(|(_, after)| after.is_alphanumeric());
                if !word && !apostrophe {
                    break;
                }
                chars.next();
                end = at + next.len_utf8();
            }
        }
        if tokens.len() >= MAX_ITEMS {
            return Err(Error::Capacity);
        }
        let value = &text[start..end];
        let stem = ["'s", "'S", "’s", "’S"]
            .into_iter()
            .find_map(|s| value.strip_suffix(s))
            .unwrap_or(value);
        tokens.push(Token {
            range: start..end,
            text: stem.into(),
        });
    }
    Ok(tokens)
}
