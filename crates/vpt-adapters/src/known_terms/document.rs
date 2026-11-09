use super::{MAX_BYTES, MAX_TERMS, TermsError};
use vpt_domain::normalize::canonical;

pub(crate) fn lines(bytes: &[u8]) -> Result<Vec<&str>, TermsError> {
    if bytes.len() > MAX_BYTES {
        return Err(TermsError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| TermsError::InvalidText)?;
    if text
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(TermsError::InvalidText);
    }
    let mut terms = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if terms.len() == MAX_TERMS || line.chars().count() > 65_536 {
            return Err(TermsError::TooLarge);
        }
        terms.push(line);
    }
    Ok(terms)
}

pub(crate) fn append(bytes: &[u8], term: &str) -> Result<Vec<u8>, TermsError> {
    let terms = lines(bytes)?;
    let key = key(term);
    if terms.iter().any(|term| self::key(term) == key) {
        return Ok(bytes.to_vec());
    }
    let separator = !bytes.is_empty() && !bytes.ends_with(b"\n");
    if terms.len() == MAX_TERMS || bytes.len() + term.len() + usize::from(separator) + 1 > MAX_BYTES
    {
        return Err(TermsError::TooLarge);
    }
    let mut result = bytes.to_vec();
    if separator {
        result.push(b'\n');
    }
    result.extend_from_slice(term.as_bytes());
    result.push(b'\n');
    Ok(result)
}

fn key(term: &str) -> String {
    canonical(term)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
