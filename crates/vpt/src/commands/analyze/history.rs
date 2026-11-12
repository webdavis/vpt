use super::prepare::Prepared;
use vpt_domain::{
    identity::RecordingId,
    run::{EffectKind, EffectReference, HistoryFact},
    time::UtcInstant,
};

pub(super) fn facts(
    prepared: &Prepared,
    id: &RecordingId,
    at: UtcInstant,
) -> Vec<(UtcInstant, HistoryFact)> {
    let delta = &prepared.delta;
    let mut facts = crate::commands::note::history::facts(delta, id, at);
    if let Some(value) = &delta.vocabulary {
        facts.push((
            at,
            HistoryFact::EffectIntent {
                kind: EffectKind::Publication,
                reference: EffectReference::PublicationTarget(value.entry.target.clone()),
            },
        ));
    }
    facts.extend(prepared.rejections.iter().map(|rejected| {
        (
            at,
            HistoryFact::TagRejected {
                recording: id.clone(),
                index: rejected.index as u32,
                reason: rejected.reason,
            },
        )
    }));
    facts.extend(prepared.renamed.iter().map(|index| {
        (
            at,
            HistoryFact::TagRenamed {
                recording: id.clone(),
                index: *index as u32,
            },
        )
    }));
    facts
}
