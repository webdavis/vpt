use vpt_adapters::{known_tags, note_index::Index};
use vpt_application::{
    Settings,
    ports::{TagLedger, VocabularyKind},
    tag_hints::{self, TagHints},
};
use vpt_domain::tags::{TagError, Vocabulary};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

pub(crate) fn read(
    settings: &Settings,
    index: &Index,
) -> Result<(Vocabulary, usize), Box<ErrorDocument>> {
    let mut tags = known_tags::read(&settings.tags.known_tags_path).map_err(|error| {
        crate::compose::publication::vocabulary::document(VocabularyKind::Tags, error)
    })?;
    let mut diagnostics = index.diagnostics().len();
    let mut bytes: usize = tags.iter().map(String::len).sum();
    for tag in index.notes().iter().flat_map(|note| &note.metadata.tags) {
        if vpt_domain::tags::key(tag).is_err() {
            diagnostics += 1;
            continue;
        }
        bytes += tag.len();
        if tags.len() == 65_536 || bytes > 16 * 1024 * 1024 {
            return Err(error(TagError::CapacityExceeded));
        }
        tags.push(tag.clone());
    }
    Ok((Vocabulary::new(&tags).map_err(error)?, diagnostics))
}

pub(crate) fn hints(
    settings: &Settings,
    index: &Index,
    ledger: &impl TagLedger,
) -> Result<TagHints, Box<ErrorDocument>> {
    if !tag_hints::wanted(&settings.analysis.command) {
        return Ok(TagHints::default());
    }
    let vocabulary = known_tags::read(&settings.tags.known_tags_path).map_err(|error| {
        crate::compose::publication::vocabulary::document(VocabularyKind::Tags, error)
    })?;
    let ledger_error = |error| Box::new(crate::compose::errors::ledger_error(error));
    Ok(tag_hints::hints(
        index
            .notes()
            .iter()
            .map(|note| note.metadata.tags.as_slice()),
        &vocabulary,
        &ledger.tag_states().map_err(ledger_error)?,
        &ledger.tag_renames().map_err(ledger_error)?,
    ))
}

fn error(error: TagError) -> Box<ErrorDocument> {
    Box::new(match error {
        TagError::CapacityExceeded => {
            ErrorDocument::new(ErrorKind::Refused, "tag vocabulary exceeds its limits")
                .rule("tag_vocabulary_limit")
        }
        TagError::InvalidVocabulary => {
            ErrorDocument::new(ErrorKind::Refused, "tag vocabulary is invalid")
                .rule("invalid_tag_vocabulary")
        }
    })
}
