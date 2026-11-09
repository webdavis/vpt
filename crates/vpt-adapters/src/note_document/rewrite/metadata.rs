use super::*;

pub(in crate::note_document) struct View<'a, 'b> {
    pub source: &'a str,
    pub header: Range<usize>,
    pub tree: &'b tree::Tree<'a>,
    pub fields: &'b [Field],
    pub tags: &'b [String],
}

impl<'a, 'b> From<&'b Document<'a>> for View<'a, 'b> {
    fn from(document: &'b Document<'a>) -> Self {
        Self {
            source: document.source,
            header: document.header.clone(),
            tree: &document.tree,
            fields: &document.fields,
            tags: &document.tags,
        }
    }
}

pub(in crate::note_document) fn rewrite(
    source: &View<'_, '_>,
    replacement: &View<'_, '_>,
    previous_tags: &[String],
) -> Result<String, NoteError> {
    let mut values = HashMap::new();
    let mut order = Vec::new();
    let mut remaining = MAX_HEADER_BYTES;
    for field in replacement.fields {
        if field.key.starts_with("vpt") {
            let value = replacement.tree.flow(field.value_node)?;
            remaining = remaining
                .checked_sub(value.len() + field.key.len())
                .ok_or(NoteError::CapacityExceeded)?;
            values.insert(field.key.as_str(), value);
            order.push(field.key.as_str());
        }
    }
    if previous_tags.len() > 256
        || previous_tags
            .iter()
            .try_fold(0usize, |size, tag| size.checked_add(tag.len()))
            .is_none_or(|size| size > MAX_HEADER_BYTES)
    {
        return Err(NoteError::CapacityExceeded);
    }
    let previous: HashSet<_> = previous_tags.iter().map(String::as_str).collect();
    let mut tags: Vec<&str> = source
        .tags
        .iter()
        .map(String::as_str)
        .filter(|tag| !previous.contains(tag))
        .collect();
    for tag in replacement.tags {
        if !tags.contains(&tag.as_str()) {
            tags.push(tag);
        }
    }
    if tags.len() > 256 {
        return Err(NoteError::CapacityExceeded);
    }
    let tags = tree::string_list(&tags)?;
    if tags.len() > remaining {
        return Err(NoteError::CapacityExceeded);
    }
    values.insert("tags", tags);
    order.push("tags");
    let yaml = &source.source[source.header.clone()];
    let flow = yaml
        .as_bytes()
        .get(source.tree.nodes[source.tree.root].span.start)
        == Some(&b'{');
    let mut edits = Vec::new();
    for (index, field) in source.fields.iter().enumerate() {
        if let Some(value) = values.remove(field.key.as_str()) {
            let node = &source.tree.nodes[field.value_node];
            for range in properties::tags(
                yaml,
                source.tree.nodes[field.key_node].span.end..node.span.start,
            ) {
                edits.push(Edit {
                    range,
                    text: String::new(),
                });
            }
            if !values::unchanged(source.tree, field.value_node, &value)? {
                edits.push(Edit {
                    range: node.span.start
                        ..node.span.start
                            + yaml[node.span.clone()]
                                .trim_end_matches([' ', '\t', '\r', '\n'])
                                .len(),
                    text: value,
                });
            }
        } else if field.key.starts_with("vpt") {
            edits.extend(
                remove::field(source, index, flow)?
                    .into_iter()
                    .map(|range| Edit {
                        range,
                        text: String::new(),
                    }),
            );
        }
    }
    let mut header = edits::apply(yaml, edits)?;
    let mut additions = Vec::new();
    for key in order {
        if let Some(value) = values.remove(key) {
            additions.push(format!("{}: {value}", tree::quote(key)?));
        }
    }
    additions::append(&mut header, &additions, flow)?;
    Ok(header)
}
