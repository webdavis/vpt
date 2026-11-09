use super::RenderError;
use crate::identity::RecordingId;

pub fn transcript_name(
    recording: &RecordingId,
    title: Option<&str>,
    maximum: usize,
) -> Result<String, RenderError> {
    let slug = sanitized_title(title.unwrap_or_default(), maximum)?;
    Ok(format!(
        "{}-{slug}-{}.md",
        recording.capture_date(),
        &recording.hash12()[..8]
    ))
}

pub fn sanitized_title(title: &str, maximum: usize) -> Result<String, RenderError> {
    if maximum == 0 {
        return Err(RenderError::InvalidName);
    }
    if title.len() > 4 * 65_536 || title.chars().count() > 65_536 {
        return Err(RenderError::CapacityExceeded);
    }
    let mut slug = String::new();
    let mut separator = false;
    for ch in crate::normalize::canonical(title).chars() {
        if ch.is_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch);
            separator = false;
        } else {
            separator = true;
        }
    }
    if let Some((boundary, next)) = slug.char_indices().nth(maximum) {
        let prefix = &slug[..boundary];
        let end = if next == '-' {
            boundary
        } else {
            prefix.rfind('-').unwrap_or(boundary)
        };
        slug.truncate(end);
    }
    if slug.is_empty() {
        slug.push_str("untitled");
    }
    Ok(slug)
}

#[cfg(test)]
mod tests;
