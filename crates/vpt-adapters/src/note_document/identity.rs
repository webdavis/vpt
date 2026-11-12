use super::*;

pub(super) fn recording(source: &str) -> Result<Option<RecordingId>, NoteError> {
    if source.len() > vpt_domain::note::MAX_RENDERED {
        return Err(NoteError::CapacityExceeded);
    }
    if source.trim_start_matches('\u{feff}').lines().next() != Some("---") {
        return Ok(None);
    }
    let (header, _) = header::locate(source)?;
    let tree = tree::Tree::parse(&source[header])?;
    let fields = header::fields(&tree)?;
    fields
        .iter()
        .find(|field| field.key == "vpt-recording")
        .map(|field| {
            RecordingId::parse(tree.string(field.value_node)?).map_err(|_| NoteError::Identity)
        })
        .transpose()
}
