mod graph;
use super::{direct::spend, *};
use crate::tags;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn rejection(row: &Candidate, request: &Request<'_>) -> Option<Rejection> {
    if row.source.is_none() {
        return Some(Rejection::MissingTranscript);
    }
    let lower = i128::from(request.now.secs) - i128::from(request.options.lookback_days) * 86_400;
    if row.captured_at > request.now {
        Some(Rejection::FutureCapture)
    } else if i128::from(row.captured_at.secs) < lower {
        Some(Rejection::OutsideLookback)
    } else {
        None
    }
}

pub(super) fn expand(
    request: &Request<'_>,
    indices: &[usize],
    matches: &mut [Option<Match>],
    remaining: &mut usize,
) -> Result<(), BriefError> {
    let components = graph::groups(request.candidates, indices)?;
    let mut chains: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut tags: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut keys = vec![Vec::new(); request.candidates.len()];
    let mut active = VecDeque::new();
    for index in indices {
        let row = &request.candidates[*index];
        if matches[*index].is_some() {
            active.push_back(*index);
        } else if rejection(row, request).is_some() {
            continue;
        }
        chains.entry(components[*index]).or_default().push(*index);
        let mut unique = BTreeSet::new();
        for tag in &row.tags {
            if tag.state == tags::State::Confirmed {
                let key = tags::key(&tag.name).map_err(|_| BriefError::InvalidInput)?;
                spend(remaining, key.len() + 1)?;
                unique.insert(key);
            }
        }
        for key in unique {
            tags.entry(key.clone()).or_default().push(*index);
            keys[*index].push(key);
        }
    }
    while let Some(anchor) = active.pop_front() {
        if let Some(group) = chains.remove(&components[anchor]) {
            activate(
                group,
                "continues",
                anchor,
                request,
                matches,
                &mut active,
                remaining,
            )?;
        }
        for key in &keys[anchor] {
            if let Some(group) = tags.remove(key) {
                activate(group, key, anchor, request, matches, &mut active, remaining)?;
            }
        }
    }
    Ok(())
}

fn activate(
    group: Vec<usize>,
    value: &str,
    anchor: usize,
    request: &Request<'_>,
    matches: &mut [Option<Match>],
    active: &mut VecDeque<usize>,
    remaining: &mut usize,
) -> Result<(), BriefError> {
    for index in group {
        if matches[index].is_none() {
            spend(remaining, value.len() + 31)?;
            matches[index] = Some(Match {
                selector: Selector::Recent,
                values: vec![value.into()],
                anchors: vec![request.candidates[anchor].recording.clone()],
            });
            active.push_back(index);
        }
    }
    Ok(())
}
