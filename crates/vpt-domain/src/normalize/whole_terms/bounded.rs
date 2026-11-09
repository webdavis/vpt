use super::MatchError;

pub(super) fn normalize(values: &[&str]) -> Result<Vec<String>, MatchError> {
    if values.len() > 65_536 {
        return Err(MatchError::CapacityExceeded);
    }
    let mut raw_bytes = 16 * 1024 * 1024usize;
    let mut normalized_bytes = raw_bytes;
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        bound(value)?;
        raw_bytes = raw_bytes
            .checked_sub(value.len())
            .ok_or(MatchError::CapacityExceeded)?;
        let value = crate::normalize::canonical(value);
        bound(&value)?;
        normalized_bytes = normalized_bytes
            .checked_sub(value.len())
            .ok_or(MatchError::CapacityExceeded)?;
        result.push(value);
    }
    Ok(result)
}

fn bound(value: &str) -> Result<(), MatchError> {
    if value.len() > 4 * 65_536 || value.chars().count() > 65_536 {
        return Err(MatchError::CapacityExceeded);
    }
    Ok(())
}
