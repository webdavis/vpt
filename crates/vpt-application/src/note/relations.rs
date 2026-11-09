use crate::ports::*;
mod mentions;
use vpt_domain::{
    relations::{self, Capture, Kind, Provenance, Relation, Rule, State, Target},
    time::UtcInstant,
};

pub(super) fn prepare(
    input: &NoteInputs,
    window_minutes: u32,
    at: UtcInstant,
) -> Result<(Vec<NoteRelation>, Option<RelationChange>), super::NoteWriteError> {
    if input.relations.len() > 65_536 {
        return Err(super::NoteWriteError::Render(
            vpt_domain::note::RenderError::CapacityExceeded,
        ));
    }

    let recordings: Vec<_> = input
        .predecessor
        .iter()
        .map(|previous| capture(&previous.record))
        .collect();
    let previous = relations::continues(capture(&input.record), &recordings, window_minutes);
    let expected: Vec<_> = input
        .relations
        .iter()
        .map(|item| item.relation.clone())
        .collect();
    let mut next = input.relations.clone();
    next.retain(|item| item.relation.provenance != Provenance::Derived(Rule::ImmediatePredecessor));
    if let Some(id) = previous {
        let target = Target::Recording(id.clone());
        if !next
            .iter()
            .any(|item| item.relation.kind == Kind::Continues && item.relation.target == target)
        {
            next.push(NoteRelation {
                path: input
                    .predecessor
                    .as_ref()
                    .and_then(|previous| previous.path.clone()),
                relation: Relation {
                    kind: Kind::Continues,
                    target,
                    state: State::Confirmed,
                    provenance: Provenance::Derived(Rule::ImmediatePredecessor),
                    ranges: vec![],
                },
            });
        }
    }
    let ambiguities = mentions::merge(input, &mut next)?;
    next.sort_by(|a, b| {
        (a.relation.kind, a.relation.target.as_str())
            .cmp(&(b.relation.kind, b.relation.target.as_str()))
    });
    let relations: Vec<_> = next.iter().map(|item| item.relation.clone()).collect();
    let change = (relations != expected || ambiguities.is_some()).then(|| RelationChange {
        ambiguities,
        recording: input.record.id.clone(),
        expected,
        relations,
        at,
    });
    Ok((next, change))
}

fn capture(record: &RecordingRecord) -> Capture<'_> {
    Capture {
        id: &record.id,
        at: record.captured_at,
        duration_secs: record.duration_secs,
    }
}
