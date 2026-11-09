use super::*;

pub(super) fn unchanged(tree: &tree::Tree<'_>, id: usize, value: &str) -> Result<bool, NoteError> {
    Ok(!contains_alias(tree, id) && tree.flow(id)? == value)
}

fn contains_alias(tree: &tree::Tree<'_>, id: usize) -> bool {
    match &tree.nodes[id].kind {
        tree::Kind::Alias(_) => true,
        tree::Kind::Sequence(children) | tree::Kind::Mapping(children) => {
            children.iter().any(|child| contains_alias(tree, *child))
        }
        tree::Kind::Scalar { .. } => false,
    }
}
