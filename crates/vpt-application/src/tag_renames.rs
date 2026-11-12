use crate::ports::TagRename;
use std::collections::HashSet;
use vpt_domain::tags::{State, proposed_key};

pub fn active<'a>(renames: &'a [TagRename], saved: &[(String, State)]) -> Vec<&'a TagRename> {
    let confirmed: HashSet<_> = saved
        .iter()
        .filter(|(_, state)| *state == State::Confirmed)
        .filter_map(|(name, _)| proposed_key(name).ok())
        .collect();
    renames
        .iter()
        .enumerate()
        .filter(|(index, rename)| {
            let old = proposed_key(&rename.old).ok();
            !old.as_ref().is_some_and(|old| confirmed.contains(old))
                && !renames[index + 1..]
                    .iter()
                    .any(|later| proposed_key(&later.new).ok() == old)
        })
        .map(|(_, rename)| rename)
        .collect()
}

pub fn follow(active: &[&TagRename], name: &str) -> Option<String> {
    let mut current = proposed_key(name).ok()?;
    let mut seen = HashSet::new();
    let mut result = None;
    while let Some(rename) = active
        .iter()
        .find(|rename| proposed_key(&rename.old).ok().as_ref() == Some(&current))
    {
        if !seen.insert(current) {
            return None;
        }
        result = Some(rename.new.clone());
        current = proposed_key(&rename.new).ok()?;
    }
    result
}
