use vpt_application::ports::BriefArtifact;
use vpt_domain::{history::*, time::UtcInstant};

pub(super) fn event(brief: &BriefArtifact, at: UtcInstant) -> Event {
    let snapshot = &brief.snapshot;
    let count = |value: usize| u32::try_from(value).unwrap_or(u32::MAX);
    Event {
        kind: EventKind::BriefWritten,
        state: EventState::Done,
        recording: None,
        occasion: Some(snapshot.id.clone()),
        detail: EventDetail::Brief {
            notes: count(snapshot.pack.sources.len()),
            spans: count(snapshot.pack.items.len()),
            tasks: count(snapshot.tasks.len()),
            unresolved: count(snapshot.pack.unresolved.flags),
        },
        counts: EventCounts::default(),
        paths: vec![EventPath {
            key: EventPathKey::Brief,
            path: brief.target.clone(),
        }],
        occurred_at: at,
    }
}
