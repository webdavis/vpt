use super::{Request, errors};
use crate::{commands::note, compose::publication};
use vpt_adapters::{FilesystemStores, Roots, SystemClock, known_terms};
use vpt_application::{Settings, ports::*};
use vpt_domain::{run::*, time::UtcInstant};
use vpt_protocol::error::ErrorDocument;

pub(super) fn prepare(
    request: &Request<'_>,
    mut inputs: NoteInputs,
    settings: &Settings,
    roots: &Roots,
    stores: &FilesystemStores,
    at: UtcInstant,
) -> Result<WorkDelta, Box<ErrorDocument>> {
    let resolution = vpt_application::review::resolve::prepare(
        &mut inputs.review,
        &request.id,
        request.flag,
        request.action,
        at,
    )
    .map_err(errors::document)?;
    let mentions = crate::compose::mentions::Context::read(settings, roots)?;
    mentions.enrich(&mut inputs, resolution.known_term.as_deref())?;
    let mut delta = vpt_application::note::prepare(
        &inputs,
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
    .map_err(note::errors::document)?;
    delta.resolution = resolution.change;
    if let Some(term) = resolution.known_term {
        let update = known_terms::prepare(&settings.reconcile.known_terms_path, &term)
            .map_err(publication::vocabulary::term_error)?;
        if update.added() {
            delta.vocabulary = Some(VocabularyPublication {
                kind: vpt_application::ports::VocabularyKind::Terms,
                entry: update.publication(at),
                bytes: update.content().to_vec(),
            });
        }
    }
    Ok(delta)
}

pub(super) fn facts(
    delta: &WorkDelta,
    request: &Request<'_>,
    at: UtcInstant,
) -> Vec<(UtcInstant, HistoryFact)> {
    let mut facts = note::history::facts(delta, &request.id, at);
    if let Some(change) = &delta.resolution {
        facts.push((
            at,
            HistoryFact::FlagResolved {
                recording: request.id.clone(),
                flag: request.flag,
                resolution: change
                    .resolution
                    .kind()
                    .expect("prepared resolution closes the flag"),
            },
        ));
    }
    if let Some(saved) = &delta.vocabulary {
        facts.push((
            at,
            HistoryFact::EffectIntent {
                kind: EffectKind::Publication,
                reference: EffectReference::PublicationTarget(saved.entry.target.clone()),
            },
        ));
    }
    facts
}
