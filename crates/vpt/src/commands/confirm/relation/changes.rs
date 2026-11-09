use super::{Request, target};
use crate::{
    commands::{confirm::usage, note},
    compose::mentions,
};
use vpt_adapters::{FilesystemStores, Roots, SqliteLedger, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    relations::{Provenance, Relation, State},
    time::UtcInstant,
};
use vpt_protocol::error::ErrorDocument;

pub(in crate::commands::confirm) fn prepare(
    request: &Request<'_>,
    mut input: NoteInputs,
    settings: &Settings,
    roots: &Roots,
    stores: &FilesystemStores,
    ledger: &SqliteLedger,
    at: UtcInstant,
) -> Result<(WorkDelta, bool, usize), Box<ErrorDocument>> {
    let index = crate::compose::index::read(settings, roots)?;
    let diagnostics = index.diagnostics().len();
    let (target, path) = target::resolve(request.target, request.kind, &input, ledger, &index)?;
    let confirming_path = path.clone();
    let expected: Vec<_> = input
        .relations
        .iter()
        .map(|link| link.relation.clone())
        .collect();
    let position = input
        .relations
        .iter()
        .position(|link| link.relation.kind == request.kind && link.relation.target == target);
    let provenance = position
        .map(|p| &input.relations[p].relation.provenance)
        .filter(|p| matches!(p, Provenance::Proposal { .. }))
        .cloned()
        .unwrap_or(Provenance::Manual);
    let relation = Relation {
        kind: request.kind,
        target,
        state: State::Confirmed,
        provenance,
        ranges: position.map_or_else(Vec::new, |p| input.relations[p].relation.ranges.clone()),
    };
    let changed = position.is_none_or(|p| input.relations[p].relation != relation);
    let link = NoteRelation { relation, path };
    if let Some(position) = position {
        input.relations[position] = link;
    } else {
        if input.relations.len() >= 65_536 {
            return Err(usage("relation count exceeds its limit"));
        }
        input.relations.push(link);
    }
    input.relations.sort_by(|a, b| {
        (a.relation.kind, a.relation.target.as_str())
            .cmp(&(b.relation.kind, b.relation.target.as_str()))
    });
    let mut delta = if input.has_renderable_notes() {
        mentions::Context::with_index(index, settings)?.enrich(&mut input, None)?;
        vpt_application::note::prepare(
            &input,
            settings,
            vpt_application::note::WriteOptions {
                analysis_content: None,
                roots: &roots.stores,
                regenerate_expired: false,
            },
            at,
            SystemClock.offset_at(at),
            stores,
            &vpt_adapters::note_document::NoteFormatter,
        )
        .map_err(note::errors::document)?
    } else {
        WorkDelta::default()
    };
    if delta
        .notes
        .iter()
        .any(|change| Some(&change.artifact.target) == confirming_path.as_ref())
    {
        return Err(usage("a relation cannot target its own note"));
    }
    let (relations, ambiguities) = match delta.relations.take() {
        Some(change) => (change.relations, change.ambiguities),
        None => (
            input
                .relations
                .into_iter()
                .map(|link| link.relation)
                .collect(),
            None,
        ),
    };
    delta.relations = (relations != expected || ambiguities.is_some()).then(|| RelationChange {
        recording: request.id.clone(),
        expected,
        relations,
        ambiguities,
        at,
    });
    Ok((delta, changed, diagnostics))
}
