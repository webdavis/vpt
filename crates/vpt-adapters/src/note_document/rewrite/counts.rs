use super::*;

pub(in crate::note_document) fn update(
    document: &Document<'_>,
    count: usize,
) -> Result<String, NoteError> {
    document.check_owner(&document.recording, &document.stage)?;
    if count > 65_539 {
        return Err(NoteError::CapacityExceeded);
    }
    let source = &document.source[document.header.clone()];
    let header = if let Some(field) = document
        .fields
        .iter()
        .find(|field| field.key == "vptOpenFlags")
    {
        let node = &document.tree.nodes[field.value_node];
        let mut changes: Vec<_> = properties::tags(
            source,
            document.tree.nodes[field.key_node].span.end..node.span.start,
        )
        .into_iter()
        .map(|range| Edit {
            range,
            text: String::new(),
        })
        .collect();
        changes.push(Edit {
            range: node.span.start
                ..node.span.start
                    + source[node.span.clone()]
                        .trim_end_matches([' ', '\t', '\r', '\n'])
                        .len(),
            text: count.to_string(),
        });
        edits::apply(source, changes)?
    } else {
        let mut header = source.to_owned();
        let flow = source
            .as_bytes()
            .get(document.tree.nodes[document.tree.root].span.start)
            == Some(&b'{');
        additions::append(&mut header, &[format!("vptOpenFlags: {count}")], flow)?;
        header
    };
    let size = document.source.len() - source.len() + header.len();
    if size > vpt_domain::note::MAX_RENDERED {
        return Err(NoteError::CapacityExceeded);
    }
    let mut result = String::with_capacity(size);
    result.push_str(&document.source[..document.header.start]);
    result.push_str(&header);
    result.push_str(&document.source[document.header.end..]);
    Document::parse(&result)?.check_owner(&document.recording, &document.stage)?;
    Ok(result)
}
