use super::{MAX_BYTES, MAX_TEXT, TagError};

pub(super) fn bounded(text: &str) -> Result<(), TagError> {
    if text.len() > 4 * MAX_TEXT || text.chars().count() > MAX_TEXT {
        return Err(TagError::CapacityExceeded);
    }
    Ok(())
}

pub(super) fn inputs(tags: &[String], limit: usize) -> Result<(), TagError> {
    if tags.len() > limit {
        return Err(TagError::CapacityExceeded);
    }
    let mut remaining = MAX_BYTES;
    for tag in tags {
        bounded(tag)?;
        remaining = remaining
            .checked_sub(tag.len())
            .ok_or(TagError::CapacityExceeded)?;
    }
    Ok(())
}

pub(super) fn proposal(text: &str) -> Result<String, TagError> {
    bounded(text)?;
    let canonical = crate::normalize::canonical(text);
    bounded(&canonical)?;
    Ok(canonical.split_whitespace().collect::<Vec<_>>().join("-"))
}

pub(super) fn valid(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '/'))
        && text.chars().any(|ch| !ch.is_numeric())
}

pub(super) fn key(normalized: &str) -> String {
    let mut key = String::new();
    let mut separator = false;
    for ch in normalized.chars() {
        if matches!(ch, '_' | '-' | '/') {
            if !separator {
                key.push('-');
            }
            separator = true;
        } else {
            key.push(ch);
            separator = false;
        }
    }
    key
}
