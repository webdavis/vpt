use super::*;

pub(super) fn append(
    header: &mut String,
    additions: &[String],
    flow: bool,
) -> Result<(), NoteError> {
    if additions.is_empty() {
        return Ok(());
    }
    let tree = tree::Tree::parse(header)?;
    let root = &tree.nodes[tree.root];
    let tree::Kind::Mapping(children) = &root.kind else {
        return Err(NoteError::Frontmatter);
    };
    if flow {
        let at = root.span.end.checked_sub(1).ok_or(NoteError::Frontmatter)?;
        let start = children
            .last()
            .map_or(root.span.start + 1, |id| tree.nodes[*id].span.end);
        let trailing_comma = remove::comma(header, start..at).next().is_some();
        let prefix = if children.is_empty() {
            ""
        } else if trailing_comma {
            " "
        } else {
            ", "
        };
        let size = prefix.len()
            + additions.iter().map(String::len).sum::<usize>()
            + (additions.len() - 1) * 2;
        if size > MAX_HEADER_BYTES - header.len() {
            return Err(NoteError::CapacityExceeded);
        }
        header.insert_str(at, &format!("{prefix}{}", additions.join(", ")));
    } else {
        let first = children
            .first()
            .map_or(root.span.start, |id| tree.nodes[*id].span.start);
        let start = header[..first].rfind('\n').map_or(0, |index| index + 1);
        let indent: String = header[start..first]
            .chars()
            .take_while(|ch| *ch == ' ')
            .collect();
        let newline = if header.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let size = additions.iter().map(String::len).sum::<usize>()
            + additions.len() * (indent.len() + newline.len())
            + newline.len();
        if size > MAX_HEADER_BYTES - header.len() {
            return Err(NoteError::CapacityExceeded);
        }
        if !header.ends_with('\n') {
            header.push_str(newline);
        }
        for addition in additions {
            header.push_str(&indent);
            header.push_str(addition);
            header.push_str(newline);
        }
    }
    Ok(())
}
