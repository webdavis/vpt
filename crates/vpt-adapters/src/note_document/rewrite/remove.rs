use super::*;

pub(super) fn field(
    source: &View<'_, '_>,
    index: usize,
    flow: bool,
) -> Result<Vec<Range<usize>>, NoteError> {
    let yaml = &source.source[source.header.clone()];
    let field = &source.fields[index];
    let start = source.tree.nodes[field.key_node].span.start;
    let end = source.tree.nodes[field.value_node].span.end;
    if flow {
        let separator = if let Some(next) = source.fields.get(index + 1) {
            let next = source.tree.nodes[next.key_node].span.start;
            Some(
                comma(yaml, end..next)
                    .next()
                    .ok_or(NoteError::Frontmatter)?,
            )
        } else if index > 0 {
            let previous = source.tree.nodes[source.fields[index - 1].value_node]
                .span
                .end;
            Some(
                comma(yaml, previous..start)
                    .next()
                    .ok_or(NoteError::Frontmatter)?,
            )
        } else {
            None
        };
        let mut ranges = Vec::with_capacity(2);
        ranges.push(start..end);
        if let Some(at) = separator {
            ranges.push(at..at + 1);
        }
        Ok(ranges)
    } else {
        let start = yaml[..start].rfind('\n').map_or(0, |at| at + 1);
        let end = if yaml[..end].ends_with('\n') {
            end
        } else {
            end + yaml[end..]
                .find('\n')
                .map_or(yaml.len() - end, |index| index + 1)
        };
        Ok(std::iter::once(start..end).collect())
    }
}

pub(super) fn comma(
    source: &str,
    range: Range<usize>,
) -> impl DoubleEndedIterator<Item = usize> + '_ {
    let mut comment = false;
    source[range.clone()]
        .char_indices()
        .filter_map(|(index, ch)| {
            if ch == '\n' || ch == '\r' {
                comment = false;
            }
            if ch == '#' {
                comment = true;
            }
            (ch == ',' && !comment).then_some(range.start + index)
        })
        .collect::<Vec<_>>()
        .into_iter()
}
