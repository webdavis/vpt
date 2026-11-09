use super::*;
use std::collections::HashSet;

pub(super) fn locate(source: &str) -> Result<(Range<usize>, usize), NoteError> {
    if source.len() > vpt_domain::note::MAX_RENDERED {
        return Err(NoteError::CapacityExceeded);
    }
    let start = usize::from(source.starts_with('\u{feff}')) * 3;
    let mut lines = source[start..].split_inclusive('\n');
    let first = lines.next().ok_or(NoteError::Frontmatter)?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return Err(NoteError::Frontmatter);
    }
    let begin = start + first.len();
    let mut offset = begin;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Ok((begin..offset, offset + line.len()));
        }
        offset += line.len();
        if offset - begin > MAX_HEADER_BYTES {
            return Err(NoteError::CapacityExceeded);
        }
    }
    Err(NoteError::Frontmatter)
}

pub(super) fn fields(tree: &tree::Tree<'_>) -> Result<Vec<Field>, NoteError> {
    let tree::Kind::Mapping(children) = &tree.nodes[tree.root].kind else {
        return Err(NoteError::Frontmatter);
    };
    if children.len() % 2 != 0 {
        return Err(NoteError::Frontmatter);
    }
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for pair in children.as_chunks::<2>().0 {
        let key = tree.string(pair[0])?;
        if !seen.insert(key) {
            return Err(NoteError::Frontmatter);
        }
        result.push(Field {
            key: key.into(),
            key_node: pair[0],
            value_node: pair[1],
        });
    }
    Ok(result)
}
