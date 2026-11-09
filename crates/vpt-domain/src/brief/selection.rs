use super::*;
use std::collections::BTreeSet;

pub(super) fn select(request: &Request<'_>) -> Result<Selection, BriefError> {
    let mut indices = validate::inputs(request)?;
    let terms = terms::prepare(request)?;
    let tags = request
        .occasion
        .tags
        .iter()
        .map(|tag| crate::tags::proposed_key(tag).map_err(|_| BriefError::InvalidInput))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut remaining = 32 * 1024 * 1024;
    let mut matches = vec![None; request.candidates.len()];
    for index in &indices {
        matches[*index] =
            direct::matched(&request.candidates[*index], &terms, &tags, &mut remaining)?;
    }
    recent::expand(request, &indices, &mut matches, &mut remaining)?;
    indices.sort_by(|a, b| {
        let key = |index: usize| {
            (
                matches[index].is_none(),
                matches[index].as_ref().map(|m| m.selector),
            )
        };
        key(*a)
            .cmp(&key(*b))
            .then_with(|| {
                request.candidates[*b]
                    .captured_at
                    .cmp(&request.candidates[*a].captured_at)
            })
            .then_with(|| {
                request.candidates[*a]
                    .recording
                    .as_str()
                    .cmp(request.candidates[*b].recording.as_str())
            })
    });
    let mut result = Selection {
        participants: terms.participants,
        ..Default::default()
    };
    for index in indices {
        let row = &request.candidates[index];
        let matched = matches[index].take();
        let selected = matched.is_some() && result.selected.len() < request.options.max_notes;
        let rejected = if matched.is_some() {
            None
        } else {
            Some(recent::rejection(row, request).unwrap_or(Rejection::NoMatch))
        };
        if selected {
            result.selected.push(index);
        } else if matched.is_some() {
            result.omitted_notes += 1;
        } else {
            result.rejected_notes += 1;
        }
        result.decisions.push(Decision {
            recording: row.recording.clone(),
            matched,
            rejected,
            selected,
        });
    }
    Ok(result)
}
