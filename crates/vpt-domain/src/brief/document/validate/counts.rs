use super::*;
use std::collections::BTreeSet;

pub(super) fn check(pack: &crate::brief::EvidencePack) -> Result<(), DocumentError> {
    let selection = &pack.selection;
    let omitted = selection
        .decisions
        .iter()
        .filter(|d| !d.selected && d.matched.is_some())
        .count();
    let rejected = selection
        .decisions
        .iter()
        .filter(|d| d.rejected.is_some())
        .count();
    let unique: BTreeSet<_> = selection.selected.iter().collect();
    if omitted != selection.omitted_notes
        || rejected != selection.rejected_notes
        || unique.len() != selection.selected.len()
        || pack.omitted_spans > 1_048_576
        || pack.unresolved.flags > 1_048_576
        || pack.unresolved.notes_with_flags > pack.sources.len()
        || (pack.unresolved.flags == 0) != (pack.unresolved.notes_with_flags == 0)
        || pack.unresolved.notes_with_flags > pack.unresolved.flags
    {
        return Err(DocumentError::Invalid);
    }
    Ok(())
}
