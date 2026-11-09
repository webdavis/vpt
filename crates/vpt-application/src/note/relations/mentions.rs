use crate::{note::NoteWriteError, ports::*};
use std::collections::BTreeSet;
use vpt_domain::{
    note::RenderError,
    relations::{Provenance, Rule},
};

pub(super) fn merge(
    input: &NoteInputs,
    next: &mut Vec<NoteRelation>,
) -> Result<Option<AmbiguityChange>, NoteWriteError> {
    let Some(derived) = &input.derived_mentions else {
        return Ok(None);
    };
    if derived.links.len() > 65_536 || derived.ambiguities.len() > 65_536 {
        return Err(NoteWriteError::Render(RenderError::CapacityExceeded));
    }
    next.retain(|item| item.relation.provenance != Provenance::Derived(Rule::KnownTerm));
    let mut keys: BTreeSet<_> = next
        .iter()
        .map(|item| (item.relation.kind, item.relation.target.as_str().to_owned()))
        .collect();
    for item in &derived.links {
        if keys.insert((item.relation.kind, item.relation.target.as_str().to_owned())) {
            if next.len() == 65_536 {
                return Err(NoteWriteError::Render(RenderError::CapacityExceeded));
            }
            next.push(item.clone());
        }
    }
    let mut ambiguities = derived.ambiguities.clone();
    ambiguities.sort_by(|a, b| a.term.cmp(&b.term));
    Ok(
        (ambiguities != input.mention_ambiguities).then(|| AmbiguityChange {
            expected: input.mention_ambiguities.clone(),
            ambiguities,
        }),
    )
}
