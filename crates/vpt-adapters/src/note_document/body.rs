use super::{NoteError, header};
/// Returns the body byte for byte after an optional frontmatter block.
pub fn body(source: &str) -> Result<&str, NoteError> {
    let first = source
        .trim_start_matches('\u{feff}')
        .split('\n')
        .next()
        .unwrap_or_default()
        .trim_end_matches('\r');
    if first != "---" {
        return Ok(source);
    }
    let (_, offset) = header::locate(source)?;
    Ok(&source[offset..])
}
