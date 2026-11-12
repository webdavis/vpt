use super::Context;
use crate::compose::errors::ledger_error;
use std::collections::HashSet;
use vpt_application::ports::{NoteInputs, TagLedger};
use vpt_domain::tags::{self, Tag};
use vpt_protocol::error::ErrorDocument;

/// Keys of the tags this redo dropped that synthesis once learned and that nothing else carries.
pub(super) fn candidates(
    previous: Option<&[Tag]>,
    next: &[Tag],
    input: &NoteInputs,
    context: &Context<'_>,
) -> Result<HashSet<String>, Box<ErrorDocument>> {
    let kept = keys(next.iter().map(|tag| tag.name.as_str()));
    let mut dropped: HashSet<_> = keys(
        previous
            .unwrap_or_default()
            .iter()
            .map(|tag| tag.name.as_str()),
    )
    .difference(&kept)
    .cloned()
    .collect();
    if dropped.is_empty() {
        return Ok(dropped);
    }
    let ledger_error = |error| Box::new(ledger_error(error));
    let learned: HashSet<_> = context
        .ledger
        .learned_tags()
        .map_err(ledger_error)?
        .into_iter()
        .collect();
    dropped.retain(|key| learned.contains(key));
    let others = context.ledger.tag_names().map_err(ledger_error)?;
    let own: HashSet<_> = input.notes.iter().map(|note| &note.target).collect();
    let carried = keys(
        others
            .iter()
            .filter(|(recording, _)| *recording != input.record.id)
            .map(|(_, name)| name.as_str())
            .chain(
                context
                    .mentions
                    .index()
                    .notes()
                    .iter()
                    .filter(|note| !own.contains(&note.path))
                    .flat_map(|note| note.metadata.tags.iter().map(String::as_str)),
            ),
    );
    dropped.retain(|key| !carried.contains(key));
    Ok(dropped)
}

fn keys<'a>(names: impl Iterator<Item = &'a str>) -> HashSet<String> {
    names.filter_map(|name| tags::key(name).ok()).collect()
}
