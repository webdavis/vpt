use super::{GroundingError, MAX_ITEMS};
use unicode_normalization::char::is_combining_mark;

pub(super) struct Token {
    pub start: usize,
    pub end: usize,
}
impl Token {
    pub fn text<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
}

pub(super) fn currency(ch: char) -> bool {
    "$¢£¤¥֏؋৲৳৻૱௹฿៛₠₡₢₣₤₥₦₧₨₩₪₫€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿".contains(ch)
}

pub(super) fn scan(text: &str) -> Result<Vec<Token>, GroundingError> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut previous = None;
    let mut currency_prefix = false;
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        let next = chars.peek().map(|(_, c)| *c);
        let alphanumeric = |c: char| c.is_alphanumeric();
        let numeric = |c: char| c.is_numeric();
        let keep = ch.is_alphanumeric()
            || (is_combining_mark(ch) && start.is_some())
            || (matches!(ch, '\'' | '’' | '-')
                && previous.is_some_and(alphanumeric)
                && next.is_some_and(alphanumeric))
            || (matches!(ch, '.' | ',' | ':' | '/' | '⁄' | '−' | '–' | '‐')
                && previous.is_some_and(numeric)
                && next.is_some_and(numeric))
            || (matches!(ch, '+' | '-' | '−')
                && next.is_some_and(|c| c.is_numeric() || currency(c)))
            || (ch == '.' && start.is_none() && next.is_some_and(numeric))
            || currency(ch)
            || (ch.is_whitespace() && currency_prefix)
            || (ch == '%' && previous.is_some_and(numeric));
        if keep {
            if start.is_none() {
                start = Some(index);
                currency_prefix = currency(ch)
                    && chars
                        .clone()
                        .find(|(_, next)| !next.is_whitespace())
                        .is_some_and(|(_, next)| {
                            next.is_numeric() || matches!(next, '+' | '-' | '−')
                        });
            } else if !ch.is_whitespace() {
                currency_prefix = false;
            }
        } else if let Some(start) = start.take() {
            push(&mut tokens, start, index)?;
            currency_prefix = false;
        }
        previous = Some(ch);
    }
    if let Some(start) = start {
        push(&mut tokens, start, text.len())?;
    }
    Ok(tokens)
}

fn push(tokens: &mut Vec<Token>, start: usize, end: usize) -> Result<(), GroundingError> {
    if tokens.len() == MAX_ITEMS {
        return Err(GroundingError::CapacityExceeded);
    }
    tokens.push(Token { start, end });
    Ok(())
}
