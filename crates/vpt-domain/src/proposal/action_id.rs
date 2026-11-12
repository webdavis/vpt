use super::CheckedAction;
use crate::{identity::RecordingId, search::timecode};

/// Ids never depend on wording, so a reworded action keeps its id. A sourced action is
/// `<recording id>@<mm:ss>`, with `#2`, `#3`, ... for later actions starting in the same second.
/// An action without a source is `<recording id>#<n>` by its order among those without one.
pub fn action_ids(recording: &RecordingId, actions: &[CheckedAction]) -> Vec<String> {
    let mut seen: Vec<Option<u64>> = Vec::new();
    actions
        .iter()
        .map(|action| {
            let start = action
                .claim
                .grounding
                .sources
                .first()
                .map(|source| source.start_secs);
            let order = seen.iter().filter(|earlier| **earlier == start).count() + 1;
            seen.push(start);
            match (start, order) {
                (Some(start), 1) => format!("{}@{}", recording.as_str(), timecode(start)),
                (Some(start), _) => format!("{}@{}#{order}", recording.as_str(), timecode(start)),
                (None, _) => format!("{}#{order}", recording.as_str()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
