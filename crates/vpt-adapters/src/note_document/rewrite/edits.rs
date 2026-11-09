use super::*;

pub(super) struct Edit {
    pub range: Range<usize>,
    pub text: String,
}

pub(super) fn apply(source: &str, mut edits: Vec<Edit>) -> Result<String, NoteError> {
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    let mut result = String::new();
    let mut cursor = 0;
    for edit in edits {
        if edit.range.start < cursor {
            if edit.text.is_empty() && edit.range.end >= cursor {
                cursor = edit.range.end;
                continue;
            }
            return Err(NoteError::Frontmatter);
        }
        let prefix = source
            .get(cursor..edit.range.start)
            .ok_or(NoteError::Frontmatter)?;
        if prefix.len().saturating_add(edit.text.len()) > MAX_HEADER_BYTES - result.len() {
            return Err(NoteError::CapacityExceeded);
        }
        result.push_str(prefix);
        result.push_str(&edit.text);
        cursor = edit.range.end;
    }
    let suffix = source.get(cursor..).ok_or(NoteError::Frontmatter)?;
    if suffix.len() > MAX_HEADER_BYTES - result.len() {
        return Err(NoteError::CapacityExceeded);
    }
    result.push_str(suffix);
    Ok(result)
}
