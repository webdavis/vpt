use super::*;
use blocks::Block;
use tokens::Token;

pub(super) fn claims<'a>(
    source: &'a str,
    frontmatter: bool,
    flags: Option<&[vpt_domain::review::ReviewFlag]>,
) -> Result<Claims<'a>, ClaimError> {
    let corrections = corrections::Saved::new(flags.unwrap_or_default())?;
    let mut output = Claims {
        source,
        claims: vec![],
        removals: vec![],
    };
    let mut bytes = 0usize;
    let newlines: Vec<_> = source
        .match_indices('\n')
        .map(|(offset, _)| offset)
        .collect();
    for block in blocks::read(source, frontmatter)? {
        bytes = bytes.saturating_add(block.text.len());
        if bytes > MAX_TEXT {
            return Err(ClaimError::CapacityExceeded);
        }
        let mut tokens = tokens::scan(&block, source, flags.is_none())?;
        for range in sentences::split(&block, &tokens)? {
            let first = tokens.partition_point(|token| token.range.end <= range.start);
            let last = tokens.partition_point(|token| token.range.start < range.end);
            if flags.is_some() {
                corrections.recognize(
                    &block,
                    range.clone(),
                    &mut tokens[first..last],
                    source,
                    output.claims.len() as u32,
                )?;
            }
            if let Some(mut claim) = assemble(&block, range, &tokens[first..last], source)? {
                if output.claims.len() >= MAX_ITEMS {
                    return Err(ClaimError::CapacityExceeded);
                }
                claim.index = output.claims.len() as u32;
                claim.line = newlines.partition_point(|offset| *offset < claim.start) + 1;
                output.claims.push(claim);
            }
        }
        for token in tokens.iter().filter(|token| token.annotation) {
            if output.removals.len() >= MAX_ITEMS {
                return Err(ClaimError::CapacityExceeded);
            }
            let mut raw = token.raw.clone();
            if source.as_bytes().get(raw.start.wrapping_sub(1)) == Some(&b' ') {
                raw.start -= 1;
            }
            output.removals.push(raw);
        }
    }
    output.removals.sort_by_key(|range| range.start);
    Ok(output)
}

pub(super) fn assemble(
    block: &Block,
    range: Range<usize>,
    tokens: &[Token],
    source: &str,
) -> Result<Option<ExtractedClaim>, ClaimError> {
    let mut text = String::new();
    let mut identity = String::new();
    let mut references = Vec::new();
    let mut cursor = range.start;
    for token in tokens {
        let mut start = token.range.start;
        if token.annotation
            && start > cursor
            && block.text.as_bytes()[start - 1] == b' '
            && source.as_bytes().get(token.raw.start.wrapping_sub(1)) == Some(&b' ')
        {
            start -= 1;
        }
        text.push_str(&block.text[cursor..start]);
        identity.push_str(&block.text[cursor..start]);
        if !token.annotation && token.reference {
            identity.push_str(&source[token.raw.clone()]);
            references.push(source[token.raw.start + 1..token.raw.end - 1].into());
            if text.chars().next_back().is_some_and(char::is_alphanumeric)
                && block.text[token.range.end..range.end]
                    .chars()
                    .next()
                    .is_some_and(char::is_alphanumeric)
            {
                text.push(' ');
            }
        } else if !token.annotation {
            text.push_str(&block.text[token.range.clone()]);
            identity.push_str(&block.text[token.range.clone()]);
        }
        cursor = token.range.end;
    }
    text.push_str(&block.text[cursor..range.end]);
    identity.push_str(&block.text[cursor..range.end]);
    let text = text.trim();
    let identity = identity.trim();
    if identity.is_empty() {
        return Ok(None);
    }
    if references.len() > 256
        || identity.len() > MAX_ITEMS * 4
        || identity.chars().count() > MAX_ITEMS
    {
        return Err(ClaimError::CapacityExceeded);
    }
    Ok(Some(ExtractedClaim {
        index: 0,
        claim: Claim {
            text: text.into(),
            source_references: references,
        },
        identity_text: identity.into(),
        start: block.start(range.start),
        end: tokens
            .iter()
            .filter(|token| token.annotation)
            .map(|token| token.raw.end)
            .max()
            .unwrap_or(0)
            .max(block.finish(range.end)),
        line: 0,
    }))
}
