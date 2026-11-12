use super::{super::scan::Carrier, steps::Work};
use crate::compose::{errors::ledger_error, tag_notes};
use std::path::PathBuf;
use vpt_adapters::{FilesystemStores, SystemClock, known_terms::Update};
use vpt_application::ports::*;
use vpt_domain::{history::RunId, identity::RecordingId, run::*, tags::Rename};
use vpt_protocol::error::ErrorDocument;

impl<E: EventSink> Work<'_, E> {
    pub(super) fn record(
        &mut self,
        run: RunId,
        rename: &Rename,
        known: &Update,
        stores: &FilesystemStores,
    ) -> Result<(), Box<ErrorDocument>> {
        let at = SystemClock.now();
        let mut delta = WorkDelta {
            tag_rename: Some(TagRename {
                old: rename.source().to_owned(),
                new: rename.target().to_owned(),
                at,
            }),
            ..Default::default()
        };
        let mut facts = vec![(
            at,
            HistoryFact::WorkStep {
                recording: None,
                outcome: WorkOutcome::Completed,
            },
        )];
        if known.added() {
            let entry = known.publication(at);
            facts.push((
                at,
                HistoryFact::EffectIntent {
                    kind: EffectKind::Publication,
                    reference: EffectReference::PublicationTarget(entry.target.clone()),
                },
            ));
            delta.vocabulary = Some(VocabularyPublication {
                kind: VocabularyKind::Tags,
                entry,
                bytes: known.content().to_vec(),
            });
        }
        self.commit(run, &delta, &facts, stores)
    }

    pub(super) fn recording(
        &mut self,
        run: RunId,
        carrier: Carrier,
        stores: &FilesystemStores,
    ) -> Result<(RecordingId, Vec<PathBuf>), Box<ErrorDocument>> {
        let id = carrier.id;
        let mut inputs = self
            .ledger
            .note_inputs(&id)
            .map_err(|error| Box::new(ledger_error(error)))?
            .ok_or_else(crate::commands::note::errors::not_found)?;
        let at = SystemClock.now();
        inputs.tags = Some(carrier.tags.clone());
        let mut delta = match inputs.has_renderable_notes() {
            true => tag_notes::render(
                &mut inputs,
                self.context,
                self.settings,
                self.roots,
                stores,
                at,
            )?,
            false => WorkDelta::default(),
        };
        let notes = delta
            .notes
            .iter()
            .filter(|change| change.expected_saved != Some(change.artifact.digest))
            .map(|change| change.artifact.target.clone())
            .collect();
        delta.tags = Some(TagChange {
            recording: id.clone(),
            expected: carrier.expected,
            tags: carrier.tags,
            at,
        });
        let facts = crate::commands::note::history::facts(&delta, &id, at);
        self.commit(run, &delta, &facts, stores)?;
        Ok((id, notes))
    }
}
