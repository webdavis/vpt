use super::mentions::Context;
use vpt_adapters::{FilesystemStores, Roots, SystemClock};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    tags::{Provenance, State, Tag},
    time::UtcInstant,
};
use vpt_protocol::error::ErrorDocument;

pub(crate) fn current(saved: Option<&[Tag]>, notes: &[NoteArtifact]) -> Vec<Tag> {
    saved.map(<[Tag]>::to_vec).unwrap_or_else(|| {
        notes
            .iter()
            .filter(|note| note.stage == NoteStage::Transcript)
            .flat_map(|note| &note.managed_tags)
            .map(|name| Tag {
                name: name.clone(),
                state: State::Confirmed,
                provenance: Provenance::Manual,
                updated_at: None,
            })
            .collect()
    })
}

pub(crate) fn render(
    inputs: &mut NoteInputs,
    context: &Context,
    settings: &Settings,
    roots: &Roots,
    stores: &FilesystemStores,
    at: UtcInstant,
) -> Result<WorkDelta, Box<ErrorDocument>> {
    context.enrich(inputs, None)?;
    vpt_application::note::prepare(
        inputs,
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
    .map_err(crate::commands::note::errors::document)
}
