use super::{ClaimError, MAX_ITEMS, blocks::Block, tokens::Token};
use std::ops::Range;

pub(super) fn split(block: &Block, tokens: &[Token]) -> Result<Vec<Range<usize>>, ClaimError> {
    if block.list {
        return Ok(std::iter::once(0..block.text.len()).collect());
    }
    let mut result = Vec::new();
    let text = &block.text;
    let (mut start, mut cursor, mut token) = (0, 0, 0);
    while cursor < text.len() {
        while token < tokens.len() && tokens[token].range.end <= cursor {
            token += 1;
        }
        if token < tokens.len() && tokens[token].range.start <= cursor {
            cursor = tokens[token].range.end;
            continue;
        }
        let ch = text[cursor..].chars().next().unwrap();
        cursor += ch.len_utf8();
        if !matches!(ch, '.' | '!' | '?' | '。' | '！' | '？')
            || !block.plain(cursor - ch.len_utf8())
        {
            continue;
        }
        let mut end = cursor;
        while let Some(ch) = text[end..]
            .chars()
            .next()
            .filter(|ch| matches!(ch, '"' | '\'' | '”' | '’' | ')' | ']'))
        {
            end += ch.len_utf8();
        }
        let mut next = end;
        loop {
            next += text[next..].len() - text[next..].trim_start().len();
            if let Some(token) = tokens
                .get(tokens.partition_point(|token| token.range.start < next))
                .filter(|token| token.range.start == next)
            {
                end = token.range.end;
                next = end;
            } else {
                break;
            }
        }
        let separated = next > end || next == text.len() || matches!(ch, '。' | '！' | '？');
        if separated {
            if ch == '.' && abbreviation(&text[start..cursor]) {
                continue;
            }
            if result.len() >= MAX_ITEMS {
                return Err(ClaimError::CapacityExceeded);
            }
            result.push(start..end);
            start = next;
            cursor = next;
        }
    }
    if start < text.len() {
        if result.len() >= MAX_ITEMS {
            return Err(ClaimError::CapacityExceeded);
        }
        result.push(start..text.len());
    }
    Ok(result)
}

fn abbreviation(text: &str) -> bool {
    let token = text.split_whitespace().next_back().unwrap_or("");
    matches!(
        token.to_ascii_lowercase().as_str(),
        "mr." | "mrs." | "ms." | "dr." | "prof." | "sr." | "jr." | "st." | "vs." | "e.g." | "i.e."
    ) || (token.chars().count() == 2 && token.chars().next().is_some_and(char::is_uppercase))
        || (token.matches('.').count() > 1
            && token.chars().all(|ch| ch.is_ascii_uppercase() || ch == '.'))
}
