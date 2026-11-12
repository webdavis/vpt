use super::Request;
use crate::compose::publication;
use vpt_adapters::{FilesystemStores, Roots, known_tags};
use vpt_application::{Settings, ports::*};
use vpt_domain::{
    tags::{Action, ManualError},
    time::UtcInstant,
};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(in crate::commands::confirm) fn prepare(
    request: &Request<'_>,
    mut inputs: NoteInputs,
    settings: &Settings,
    roots: &Roots,
    stores: &FilesystemStores,
    at: UtcInstant,
) -> Result<(WorkDelta, usize), Box<ErrorDocument>> {
    let index = crate::compose::index::read(settings, roots)?;
    let (vocabulary, diagnostics) = crate::compose::tag_vocabulary::read(settings, &index)?;
    let expected = inputs.tags.clone();
    let mut tags = crate::compose::tag_notes::current(expected.as_deref(), &inputs.notes);
    let edit = vocabulary
        .edit(
            &tags,
            request.value,
            request.action,
            settings.tags.options.max_per_note,
            at,
        )
        .map_err(error)?;
    let name = edit.tag.name.clone();
    match edit.position {
        Some(index) => tags[index] = edit.tag,
        None => tags.push(edit.tag),
    }
    let change = (expected.as_ref() != Some(&tags)).then(|| TagChange {
        recording: request.id.clone(),
        expected,
        tags: tags.clone(),
        at,
    });
    inputs.tags = Some(tags);
    let mut delta = if inputs.has_renderable_notes() {
        let context = crate::compose::mentions::Context::with_index(index, settings)?;
        crate::compose::tag_notes::render(&mut inputs, &context, settings, roots, stores, at)?
    } else {
        WorkDelta::default()
    };
    delta.tags = change;
    if request.action == Action::Confirm {
        let update = known_tags::prepare_confirmed(&settings.tags.known_tags_path, &name)
            .map_err(|error| publication::vocabulary::document(VocabularyKind::Tags, error))?;
        if update.added() {
            delta.vocabulary = Some(VocabularyPublication {
                kind: VocabularyKind::Tags,
                entry: update.publication(at),
                bytes: update.content().to_vec(),
            });
        }
    }
    Ok((delta, diagnostics))
}

fn error(error: ManualError) -> Box<ErrorDocument> {
    Box::new(match error {
        ManualError::Syntax => ErrorDocument::new(ErrorKind::Usage, "tag syntax is invalid"),
        ManualError::Capacity => {
            ErrorDocument::new(ErrorKind::Refused, "saved tag state exceeds its limits")
                .rule("tag_limit")
        }
        ManualError::ConfirmedLimit => {
            ErrorDocument::new(ErrorKind::Refused, "confirmed tag limit reached")
                .rule("confirmed_tag_limit")
        }
        ManualError::InvalidState => {
            ErrorDocument::new(ErrorKind::Ledger, "saved tag state is invalid")
        }
    })
}
